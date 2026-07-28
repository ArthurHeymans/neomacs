use expect_test::expect;

use super::assert_ace_popup_menu_parity;

/// Turning the mode on is the whole installation step the README describes, and
/// it has to be a *global* switch that replaces `x-popup-menu' exactly once.
/// This walks the lifecycle a user goes through: enable, enable again by
/// accident, look at the mode from another buffer, toggle off, call
/// `x-popup-menu' while off (the real function runs and renders nothing),
/// toggle back on, and finally disable.
#[test]
fn enabling_the_global_mode_advises_x_popup_menu_exactly_once_until_disabled() {
    let elisp_form = r##"(progn
  (apm-test-setup)
  (let ((observed nil))
    (push (cons :initial (apm-test-mode-state)) observed)
    (ace-popup-menu-mode 1)
    (push (cons :enabled (apm-test-mode-state)) observed)
    (ace-popup-menu-mode 1)
    (push (cons :enabled-twice (apm-test-mode-state)) observed)
    (with-temp-buffer
      (push (cons :other-buffer (apm-test-mode-state)) observed))
    (ace-popup-menu-mode 'toggle)
    (push (cons :toggled-off (apm-test-mode-state)) observed)
    (push (cons :unadvised-call
                (list (x-popup-menu t apm-test-menu)
                      (length (apm-test-renderings))))
          observed)
    (ace-popup-menu-mode 'toggle)
    (push (cons :toggled-on (apm-test-mode-state)) observed)
    (ace-popup-menu-mode -1)
    (push (cons :disabled (apm-test-mode-state)) observed)
    (setq observed (nreverse observed))
    observed))"##;

    let expect = expect![
        "OK ((:initial :advised nil :advice-count 0 :mode nil :global-value nil :buffer-local nil) (:enabled :advised t :advice-count 1 :mode t :global-value t :buffer-local nil) (:enabled-twice :advised t :advice-count 1 :mode t :global-value t :buffer-local nil) (:other-buffer :advised t :advice-count 1 :mode t :global-value t :buffer-local nil) (:toggled-off :advised nil :advice-count 0 :mode nil :global-value nil :buffer-local nil) (:unadvised-call nil 0) (:toggled-on :advised t :advice-count 1 :mode t :global-value t :buffer-local nil) (:disabled :advised nil :advice-count 0 :mode nil :global-value nil :buffer-local nil))"
    ];

    assert_ace_popup_menu_parity(elisp_form, expect);
}

/// The headline behaviour: with the mode on, a plain `x-popup-menu' call shows
/// the two-pane menu in a temporary window and returns the value of whichever
/// item the user labels.  Selecting all five items in turn pins the complete
/// label alphabet mapping -- labels run across pane boundaries in menu order --
/// while the captured rendering pins the menu the user actually sees: the title
/// in `avy-menu-title', the items unfaced and one per line, no pane headers, no
/// cursor.  Each call must consume exactly its one key, kill the menu buffer,
/// restore the window and leave the user in the buffer they were editing.
#[test]
fn every_avy_label_returns_the_value_of_the_menu_item_it_marks() {
    let elisp_form = r##"(progn
  (apm-test-setup)
  (ace-popup-menu-mode 1)
  (let ((results nil))
    (dolist (key '("a" "s" "d" "f" "g"))
      (setq unread-command-events (listify-key-sequence (kbd key)))
      (push (list key (x-popup-menu t apm-test-menu) unread-command-events)
            results))
    (setq results (nreverse results))
    (list :selections results
          :renderings (length (apm-test-renderings))
          :rendering (car (apm-test-renderings))
          :menu-buffer-left (and (get-buffer "*ace-popup-menu*") t)
          :windows (length (window-list))
          :current (buffer-name))))"##;

    let expect = expect![[
        r#"OK (:selections (("a" rename-symbol nil) ("s" rename-file nil) ("d" extract-function nil) ("f" extract-variable nil) ("g" inline-variable nil)) :renderings 5 :rendering (:buffer "*ace-popup-menu*" :text "Refactor\n\nRename symbol\nRename file\n\nExtract function\nExtract variable\nInline variable" :runs (("Refactor" . avy-menu-title) ("\n\nRename symbol\nRename file\n\nExtract function\nExtract variable\nInline variable")) :cursor nil :window-buffer "*ace-popup-menu*") :menu-buffer-left nil :windows 1 :current "*apm-work*")"#
    ]];

    assert_ace_popup_menu_parity(elisp_form, expect);
}

/// `ace-popup-menu-show-pane-header' is the package's only user option.  With
/// it set, each pane's title is printed above its items in
/// `avy-menu-pane-header'; without it the panes are only separated by a blank
/// line.  The same menu is popped up both ways with the same key, which pins
/// both renderings side by side and the fact that pane headers are decoration:
/// they are not selectable, so `d' still returns the third item either way.
#[test]
fn showing_pane_headers_changes_the_rendering_but_not_the_labels() {
    let elisp_form = r##"(progn
  (apm-test-setup)
  (ace-popup-menu-mode 1)
  (let ((observed nil))
    (setq unread-command-events (listify-key-sequence (kbd "d")))
    (push (list :without-headers ace-popup-menu-show-pane-header
                (x-popup-menu t apm-test-menu))
          observed)
    (setq ace-popup-menu-show-pane-header t)
    (setq unread-command-events (listify-key-sequence (kbd "d")))
    (push (list :with-headers ace-popup-menu-show-pane-header
                (x-popup-menu t apm-test-menu))
          observed)
    (setq observed (nreverse observed))
    (list :selections observed
          :renderings (apm-test-renderings))))"##;

    let expect = expect![[
        r#"OK (:selections ((:without-headers nil extract-function) (:with-headers t extract-function)) :renderings ((:buffer "*ace-popup-menu*" :text "Refactor\n\nRename symbol\nRename file\n\nExtract function\nExtract variable\nInline variable" :runs (("Refactor" . avy-menu-title) ("\n\nRename symbol\nRename file\n\nExtract function\nExtract variable\nInline variable")) :cursor nil :window-buffer "*ace-popup-menu*") (:buffer "*ace-popup-menu*" :text "Refactor\n\nRename\n\nRename symbol\nRename file\n\nExtract\n\nExtract function\nExtract variable\nInline variable" :runs (("Refactor" . avy-menu-title) ("\n\n") ("Rename" . avy-menu-pane-header) ("\n\nRename symbol\nRename file\n\n") ("Extract" . avy-menu-pane-header) ("\n\nExtract function\nExtract variable\nInline variable")) :cursor nil :window-buffer "*ace-popup-menu*")))"#
    ]];

    assert_ace_popup_menu_parity(elisp_form, expect);
}

/// The way this is really reached: a command the user invoked from a key calls
/// `x-popup-menu', and the label is typed as part of the same key sequence.
/// The whole interaction has to survive the command loop -- the menu appears,
/// `d' picks the third item, and afterwards the buffer the user was editing is
/// current again, unmodified, with point where it was and the menu window gone.
#[test]
fn a_command_bound_to_a_key_pops_up_the_menu_and_restores_the_work_buffer() {
    let elisp_form = r##"(progn
  (apm-test-setup)
  (ace-popup-menu-mode 1)
  (execute-kbd-macro (kbd "C-c m d"))
  (list :result apm-test-result
        :current (buffer-name)
        :text (buffer-substring-no-properties (point-min) (point-max))
        :point (point)
        :windows (length (window-list))
        :menu-buffer-left (and (get-buffer "*ace-popup-menu*") t)
        :renderings (apm-test-renderings)))"##;

    let expect = expect![[
        r#"OK (:result extract-function :current "*apm-work*" :text "Editing buffer, untouched by the menu.\n" :point 1 :windows 1 :menu-buffer-left nil :renderings ((:buffer "*ace-popup-menu*" :text "Refactor\n\nRename symbol\nRename file\n\nExtract function\nExtract variable\nInline variable" :runs (("Refactor" . avy-menu-title) ("\n\nRename symbol\nRename file\n\nExtract function\nExtract variable\nInline variable")) :cursor nil :window-buffer "*ace-popup-menu*")))"#
    ]];

    assert_ace_popup_menu_parity(elisp_form, expect);
}

/// The docstring of `ace-popup-menu' says Emacs Lisp code may call it directly,
/// and that the original `x-popup-menu' is called via ORIG-FUN when POSITION is
/// nil or MENU is a keymap or a list of keymaps.  Passing a recording ORIG-FUN
/// -- which is simply the argument the advice machinery would pass -- pins all
/// four shapes: the three fallbacks forward the untouched POSITION and MENU and
/// return whatever the original returned, the ordinary menu takes the avy path
/// and returns the selected value, and only that last one rendered anything.
#[test]
fn the_documented_fallback_shapes_hand_the_call_to_the_original_function() {
    let elisp_form = r##"(progn
  (apm-test-setup)
  (let ((keymap (make-sparse-keymap "Keymap menu")))
    (define-key keymap [item] '(menu-item "Item" ignore))
    (let ((nil-position (ace-popup-menu #'apm-test-orig-fun nil apm-test-menu))
          (keymap-menu (ace-popup-menu #'apm-test-orig-fun t keymap))
          (keymap-list (ace-popup-menu #'apm-test-orig-fun t (list keymap keymap))))
      (setq unread-command-events (listify-key-sequence (kbd "s")))
      (let ((avy-path (ace-popup-menu #'apm-test-orig-fun t apm-test-menu)))
        (list :nil-position nil-position
              :keymap-menu keymap-menu
              :keymap-list keymap-list
              :avy-path avy-path
              :orig-calls (apm-test-orig-calls)
              :renderings (length (apm-test-renderings)))))))"##;

    let expect = expect![[
        r#"OK (:nil-position value-from-orig-fun :keymap-menu value-from-orig-fun :keymap-list value-from-orig-fun :avy-path rename-file :orig-calls ((:orig nil ("Refactor" ("Rename" ("Rename symbol" . rename-symbol) ("Rename file" . rename-file)) ("Extract" ("Extract function" . extract-function) ("Extract variable" . extract-variable) ("Inline variable" . inline-variable)))) (:orig t #1=(keymap (item menu-item "Item" ignore) "Keymap menu")) (:orig t (#1# #1#))) :renderings 1)"#
    ]];

    assert_ace_popup_menu_parity(elisp_form, expect);
}

/// Backing out: `C-g' and `ESC' are avy's escape keys, and the documented
/// contract is that a cancelled menu returns nil.  Both must leave no trace --
/// the menu window is gone, its buffer killed, the work buffer current and
/// unchanged -- and neither may leak the escape key back into the command
/// stream.
#[test]
fn cancelling_the_menu_returns_nil_and_leaves_no_window_or_buffer_behind() {
    let elisp_form = r##"(progn
  (apm-test-setup)
  (ace-popup-menu-mode 1)
  (let ((results nil))
    (dolist (key '("C-g" "ESC"))
      (setq unread-command-events (listify-key-sequence (kbd key)))
      (push (list key
                  (condition-case failure (x-popup-menu t apm-test-menu)
                    (error (list :error failure))
                    (quit (list :quit failure)))
                  unread-command-events)
            results))
    (setq results (nreverse results))
    (list :aborts results
          :renderings (length (apm-test-renderings))
          :menu-buffer-left (and (get-buffer "*ace-popup-menu*") t)
          :windows (length (window-list))
          :current (buffer-name)
          :text (buffer-substring-no-properties (point-min) (point-max)))))"##;

    let expect = expect![[
        r#"OK (:aborts (("C-g" nil nil) ("ESC" nil nil)) :renderings 2 :menu-buffer-left nil :windows 1 :current "*apm-work*" :text "Editing buffer, untouched by the menu.\n")"#
    ]];

    assert_ace_popup_menu_parity(elisp_form, expect);
}

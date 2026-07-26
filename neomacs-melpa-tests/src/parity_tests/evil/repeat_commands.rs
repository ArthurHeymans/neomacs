use expect_test::expect;

use super::{assert_evil_parity, assert_evil_signal_parity};

#[test]
fn evil_normalize_repeat_info_concatenates_adjacent_key_arrays_around_symbols() {
    let elisp_form = r##"(mapcar
               #'evil-normalize-repeat-info
               '(("abc")
                 ("M-f")
                 (SYM)
                 ("abc" [XX YY] "def")
                 (BEG MID END)
                 (BEG "abc" [XX YY] "def")
                 ("abc" [XX YY] "def" END)
                 ("abc" [XX YY] MID "def")
                 (BEG "abc" [XX YY] MID "def" END)))"##;
    let expect = expect![
        "OK (([97 98 99]) ([77 45 102]) (SYM) ([97 98 99 XX YY 100 101 102]) (BEG MID END) (BEG [97 98 99 XX YY 100 101 102]) ([97 98 99 XX YY 100 101 102] END) ([97 98 99 XX YY] MID [100 101 102]) (BEG [97 98 99 XX YY] MID [100 101 102] END))"
    ];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_dot_repeat_replays_replace_delete_insert_and_change_commands() {
    let elisp_form = r##"(mapcar
               (lambda (case)
                 (with-temp-buffer
                   (insert "one two three four")
                   (goto-char (point-min))
                   (evil-local-mode 1)
                   (evil-normal-state)
                   (execute-kbd-macro (kbd (car case)))
                   (execute-kbd-macro (kbd (cdr case)))
                   (list
                    (buffer-string)
                    (point)
                    evil-state)))
               '(("r X" . "w .")
                 ("dw" . "w .")
                 ("i X ESC" . "w .")
                 ("cw X ESC" . "w .")))"##;
    let expect = expect![[
        r#"OK (("rXw." 5 nil) ("rXw.dww." 9 nil) ("rXw.dww.iXw." 13 nil) ("rXw.dww.iXw.cwXw." 18 nil))"#
    ]];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_keypress_parser_handles_counts_operators_zero_and_incomplete_input() {
    let elisp_form = r##"(with-temp-buffer
               (evil-local-mode 1)
               (evil-operator-state)
               (list
                (evil-keypress-parser '(?d))
                (evil-keypress-parser '(?2 ?d))
                (evil-keypress-parser '(?2 ?0 ?2 ?d))
                (evil-keypress-parser '(?4 ?0 ?4 ?g ??))
                (evil-keypress-parser '(?0))
                (let ((unread-command-events '(?d)))
                  (evil-keypress-parser '(?2)))))"##;
    let expect = expect![
        "OK ((evil-delete nil) (evil-delete 2) (evil-delete 202) (evil-rot13 404) (evil-beginning-of-line nil) (evil-delete 2))"
    ];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_command_properties_add_replace_remove_and_declare_behavior_flags() {
    let elisp_form = r##"(progn
               (defun neomacs-evil-property-command ()
                 (interactive))
               (evil-set-command-properties
                'neomacs-evil-property-command
                :type 'exclusive
                :repeat nil)
               (let ((initial
                      (evil-command-properties
                       'neomacs-evil-property-command)))
                 (evil-add-command-properties
                  'neomacs-evil-property-command
                  :repeat t :keep-visual t)
                 (let ((added
                        (evil-command-properties
                         'neomacs-evil-property-command)))
                   (evil-remove-command-properties
                    'neomacs-evil-property-command :type)
                   (let ((removed
                          (evil-command-properties
                           'neomacs-evil-property-command)))
                     (evil-declare-motion
                      'neomacs-evil-property-command)
                     (list
                      initial
                      added
                      removed
                      (evil-command-properties
                       'neomacs-evil-property-command)
                      (evil-get-command-property
                       'neomacs-evil-property-command
                       :missing 'fallback)
                      (evil-has-command-property-p
                       'neomacs-evil-property-command
                       :keep-visual))))))"##;
    let expect = expect![
        "OK (#1=(:type exclusive :repeat t :keep-visual t) #1# #2=(:repeat motion . #3=(:keep-visual t)) #2# fallback #3#)"
    ];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_markers_store_local_positions_advance_flags_and_raw_marker_objects() {
    let elisp_form = r##"(with-temp-buffer
               (insert "alpha\nbeta\ngamma")
               (let ((evil-markers-alist nil))
                 (goto-char 3)
                 (evil-set-marker ?a)
                 (goto-char 8)
                 (evil-set-marker ?b nil t)
                 (let ((raw-a (evil-get-marker ?a t))
                       (raw-b (evil-get-marker ?b t)))
                   (list
                    (evil-get-marker ?a)
                    (evil-get-marker ?b)
                    (markerp raw-a)
                    (marker-insertion-type raw-a)
                    (markerp raw-b)
                    (marker-insertion-type raw-b)
                    (evil-get-marker ?z)
                    (mapcar #'car evil-markers-alist)))))"##;
    let expect = expect!["OK (3 8 t nil t t nil (98 97))"];

    assert_evil_parity(elisp_form, expect);
}

#[test]
fn evil_set_marker_rejects_a_read_only_special_marker() {
    let elisp_form = r##"(with-temp-buffer
               (evil-set-marker ?<))"##;
    let expect = expect!["ERR (wrong-type-argument markerp nil)"];

    assert_evil_signal_parity(elisp_form, expect);
}

#[test]
fn evil_yank_helpers_attach_character_line_and_rectangle_handlers_exactly() {
    let elisp_form = r##"(with-temp-buffer
               (insert "alpha\nbravo\ncharlie\n")
               (let ((kill-ring nil)
                     (kill-ring-yank-pointer nil))
                 (evil-yank-characters 1 6)
                 (let ((characters
                        (list
                         (car kill-ring)
                         (get-text-property
                          0 'yank-handler (car kill-ring)))))
                   (evil-yank-lines 1 7)
                   (let ((lines
                          (list
                           (car kill-ring)
                           (get-text-property
                            0 'yank-handler (car kill-ring)))))
                     (evil-yank-rectangle 1 9)
                     (list
                      characters
                      lines
                      (car kill-ring)
                      (get-text-property
                       0 'yank-handler (car kill-ring)))))))"##;
    let expect = expect![[
        r#"OK (("alpha" nil) (#("alpha\n" 0 6 (yank-handler (evil-yank-line-handler nil t))) (evil-yank-line-handler nil t)) #("al\nbr" 0 5 (yank-handler (evil-yank-block-handler ("al" "br") t evil-delete-yanked-rectangle))) (evil-yank-block-handler ("al" "br") t evil-delete-yanked-rectangle))"#
    ]];

    assert_evil_parity(elisp_form, expect);
}

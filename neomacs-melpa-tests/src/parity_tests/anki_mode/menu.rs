use expect_test::expect;

use super::assert_anki_mode_parity;

#[test]
fn anki_mode_initial_load_truth_table_requires_both_datasets() {
    let elisp_form = r##"(mapcar
         (lambda (state)
           (let ((anki-mode--decks (car state))
                 (anki-mode--card-types (cadr state)))
             (list state (anki-mode-initial-load-done-p))))
         '((nil nil)
           (("Default") nil)
           (nil (("Basic" "Front" "Back")))
           (("Default") (("Basic" "Front" "Back")))))"##;
    let expect = expect![[
        r#"OK (((nil nil) nil) ((("Default") nil) nil) ((nil (("Basic" "Front" "Back"))) nil) ((("Default") #1=(("Basic" "Front" "Back"))) #1#))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_menu_buffer_is_singleton_and_initialized_once() {
    let elisp_form = r##"(let ((first (anki-mode-menu-buffer))
               second)
         (with-current-buffer first
           (let ((inhibit-read-only t))
             (insert "state")))
         (setq second (anki-mode-menu-buffer))
         (prog1
             (list (eq first second)
                   (buffer-name first)
                   (buffer-local-value 'major-mode first)
                   (buffer-local-value 'buffer-read-only first)
                   (with-current-buffer second (buffer-string)))
           (kill-buffer first)))"##;
    let expect = expect![[r#"OK (t "*Anki*" anki-mode-menu-mode t "state")"#]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_menu_render_shows_null_defaults_and_exact_deck_order() {
    let elisp_form = r##"(with-temp-buffer
         (anki-mode-menu-mode)
         (let ((anki-mode--previous-deck nil)
               (anki-mode--previous-card-type nil)
               (anki-mode--decks '("Default" "Languages::French" "Work & Study")))
           (anki-mode-menu-render)
           (list major-mode
                 buffer-read-only
                 (buffer-string)
                 (point)
                 (count-lines (point-min) (point-max)))))"##;
    let expect = expect![[
        r#"OK (anki-mode-menu-mode t "Anki Mode\n---------------\n[n]: New card\n[a]: New card with current settings (deck: 'NULL', card type: 'NULL')\n[r]: Refresh decks list\n\n\n\nDecks\n---------------\n* Default\n* Languages::French\n* Work & Study\n" 205 13)"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_menu_render_replaces_stale_content_and_uses_previous_values() {
    let elisp_form = r##"(with-temp-buffer
         (insert "STALE")
         (anki-mode-menu-mode)
         (let ((anki-mode--previous-deck "Japanese::Mining")
               (anki-mode--previous-card-type "Basic (and reversed card)")
               (anki-mode--decks '("Japanese::Mining")))
           (anki-mode-menu-render)
           (list (buffer-string)
                 (string-match-p "STALE" (buffer-string))
                 (string-match-p "Japanese::Mining" (buffer-string))
                 (string-match-p "Basic (and reversed card)" (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ("Anki Mode\n---------------\n[n]: New card\n[a]: New card with current settings (deck: 'Japanese::Mining', card type: 'Basic (and reversed card)')\n[r]: Refresh decks list\n\n\n\nDecks\n---------------\n* Japanese::Mining\n" nil 84 115)"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_menu_reuse_key_rejects_missing_previous_options() {
    let elisp_form = r##"(with-temp-buffer
         (anki-mode-menu-mode)
         (let ((command (lookup-key anki-mode-menu-mode-map (kbd "a"))))
           (mapcar
            (lambda (state)
              (let ((anki-mode--previous-deck (car state))
                    (anki-mode--previous-card-type (cadr state)))
                (condition-case err
                    (progn (funcall command) 'unexpected-success)
                  (error (list state (car err) (cdr err))))))
            '((nil nil) ("Default" nil) (nil "Basic")))))"##;
    let expect = expect![[
        r#"OK (((nil nil) error ("Can’t reuse the previous options because no previous deck/card type is set")) (("Default" nil) error ("Can’t reuse the previous options because no previous deck/card type is set")) ((nil "Basic") error ("Can’t reuse the previous options because no previous deck/card type is set")))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_menu_reuse_key_forwards_previous_options_exactly() {
    let elisp_form = r##"(with-temp-buffer
         (anki-mode-menu-mode)
         (let ((anki-mode--previous-deck "Science")
               (anki-mode--previous-card-type "Cloze")
               calls)
           (cl-letf (((symbol-function 'anki-mode-new-card-noninteractive)
                      (lambda (&rest args) (push args calls))))
             (funcall (lookup-key anki-mode-menu-mode-map (kbd "a")))
             (list (nreverse calls)
                   anki-mode--previous-deck
                   anki-mode--previous-card-type))))"##;
    let expect = expect![[r#"OK ((("Science" "Cloze")) "Science" "Cloze")"#]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_new_card_interactive_refreshes_and_records_choices() {
    let elisp_form = r##"(let ((anki-mode--decks nil)
               (anki-mode--card-types nil)
               calls
               (answers '("Deck B" "Model Y")))
         (cl-letf (((symbol-function 'anki-mode-refresh)
                    (lambda ()
                      (push 'refresh calls)
                      (setq anki-mode--decks '("Deck A" "Deck B")
                            anki-mode--card-types
                            '(("Model X" "Front") ("Model Y" "Q" "A")))))
                   ((symbol-function 'completing-read)
                    (lambda (prompt collection &rest _)
                      (push (list prompt collection) calls)
                      (pop answers)))
                   ((symbol-function 'anki-mode-new-card-noninteractive)
                    (lambda (&rest args) (push (cons 'new-card args) calls))))
           (anki-mode-new-card)
           (list (nreverse calls)
                 anki-mode--previous-deck
                 anki-mode--previous-card-type)))"##;
    let expect = expect![[
        r#"OK ((refresh ("Choose deck: " ("Deck A" "Deck B")) ("Choose card type: " ("Model X" "Model Y")) (new-card "Deck B" "Model Y")) "Deck B" "Model Y")"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_menu_command_switches_refreshes_and_renders_in_order() {
    let elisp_form = r##"(let ((anki-mode--decks nil)
               (anki-mode--card-types nil)
               calls)
         (cl-letf (((symbol-function 'switch-to-buffer)
                    (lambda (buffer)
                      (push (list 'switch (buffer-name buffer)) calls)
                      (set-buffer buffer)))
                   ((symbol-function 'anki-mode-refresh)
                    (lambda ()
                      (push 'refresh calls)
                      (setq anki-mode--decks '("Ready")
                            anki-mode--card-types '(("Basic" "Front")))))
                   ((symbol-function 'anki-mode-menu-render)
                    (lambda () (push (list 'render (buffer-name)) calls))))
           (anki-mode-menu)
           (prog1
               (list (nreverse calls) (buffer-name) major-mode)
             (kill-buffer "*Anki*"))))"##;
    let expect = expect![[
        r#"OK (((switch "*Anki*") refresh (render "*Anki*")) "*Anki*" anki-mode-menu-mode)"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

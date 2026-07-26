use expect_test::expect;

use super::assert_abc_mode_parity;

#[test]
fn abc_insert_instrument_uses_exact_completion_contract_and_canonical_alias_name() {
    let elisp_form = r##"(let ((answers '("piano" "Flute"))
                    events)
               (with-temp-buffer
                 (cl-letf
                     (((symbol-function 'completing-read)
                       (lambda
                         (prompt collection predicate require-match
                          initial-input history &optional default)
                         (push
                          (list
                           'complete
                           prompt
                           (eq collection abc-midi-instruments-alist)
                           predicate
                           require-match
                           initial-input
                           history
                           default)
                          events)
                         (pop answers))))
                   (list
                    (abc-insert-instrument)
                    (buffer-string)
                    (progn
                      (insert "|")
                      (abc-insert-instrument "Bass"))
                    (buffer-string)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (t "0 % Acoustic Grand Piano" t "0 % Acoustic Grand Piano|73 % Flute" ((complete "MIDI instrument: " t nil t nil abc-instrument-history nil) (complete "Bass MIDI instrument: " t nil t nil abc-instrument-history nil)))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_insert_instrument_returns_nil_without_mutating_for_unknown_completion_result() {
    let elisp_form = r##"(let ((abc-midi-instruments-alist
                    '(("Known" . 7))))
               (with-temp-buffer
                 (insert "before")
                 (cl-letf
                     (((symbol-function 'completing-read)
                       (lambda (&rest _) "Missing")))
                   (list
                    (abc-insert-instrument)
                    (buffer-string)
                    (point)))))"##;
    let expect = expect![[r#"OK (nil "before" 7)"#]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_insert_note_string_handles_nil_plain_and_each_special_action() {
    let elisp_form = r##"(let (events)
               (with-temp-buffer
                 (insert "ab")
                 (goto-char (point-max))
                 (let ((abc-mouse-specials
                        (list
                         (cons
                          "SPECIAL"
                          (lambda ()
                            (push 'special events)
                            (insert "!"))))))
                   (list
                    (abc-insert-note-string nil)
                    (abc-insert-note-string "C#")
                    (abc-insert-note-string "SPECIAL")
                    (buffer-string)
                    (point)
                    (nreverse events)))))"##;
    let expect = expect![[r#"OK (nil nil nil "abC#!" 6 (special))"#]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_cursor_to_note_extracts_tokens_at_start_middle_end_and_reports_whitespace() {
    let elisp_form = r##"(with-temp-buffer
               (insert "C#  α-note\nlast")
               (let (messages)
                 (cl-letf
                     (((symbol-function 'message)
                       (lambda (text &rest arguments)
                         (let ((rendered
                                (apply #'format text arguments)))
                           (push rendered messages)
                           rendered))))
                   (list
                    (progn
                      (goto-char 1)
                      (list (abc-cursor-to-note) (point)))
                    (progn
                      (goto-char 6)
                      (list (abc-cursor-to-note) (point)))
                    (progn
                      (goto-char (point-max))
                      (list (abc-cursor-to-note) (point)))
                    (progn
                      (goto-char 3)
                      (list (abc-cursor-to-note) (point)))
                    (nreverse messages)))))"##;
    let expect = expect![[r#"OK (("C#" 3) ("α-note" 11) ("last" 16) (nil 3) ("Not a symbol."))"#]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_insert_mouse_note_other_switches_after_reading_and_inserts_exact_note() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'abc-cursor-to-note)
                     (lambda ()
                       (push '(read-note) events)
                       "α-note"))
                    ((symbol-function 'other-window)
                     (lambda (count)
                       (push (list 'other-window count) events)))
                    ((symbol-function 'abc-insert-note-string)
                     (lambda (note)
                       (push (list 'insert-note note) events)
                       'insert-result)))
                 (list
                  (abc-insert-mouse-note-other 'ignored-event)
                  (nreverse events))))"##;
    let expect =
        expect![[r#"OK (insert-result ((read-note) (other-window 1) (insert-note "α-note")))"#]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_event_to_note_uses_last_input_event_window_buffer_and_position() {
    let elisp_form = r##"(let ((last-input-event 'event)
                    events)
               (with-temp-buffer
                 (insert "zero target end")
                 (cl-letf
                     (((symbol-function 'event-start)
                       (lambda (event)
                         (push (list 'event-start event) events)
                         'position))
                      ((symbol-function 'posn-window)
                       (lambda (position)
                         (push (list 'posn-window position) events)
                         'window))
                      ((symbol-function 'posn-point)
                       (lambda (position)
                         (push (list 'posn-point position) events)
                         7))
                      ((symbol-function 'window-buffer)
                       (lambda (window)
                         (push (list 'window-buffer window) events)
                         (current-buffer)))
                      ((symbol-function 'abc-cursor-to-note)
                       (lambda ()
                         (push (list 'cursor (point)) events)
                         "target")))
                   (list
                    (abc-event-to-note)
                    (nreverse events)
                    (point)))))"##;
    let expect = expect![[
        r#"OK ("target" ((event-start event) (posn-window position) (window-buffer window) (event-start event) (posn-point position) (cursor 7)) 7)"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_mouse_populates_new_note_buffer_once_binds_mouse_and_returns_to_editor() {
    let elisp_form = r##"(let ((abc-mouse-pad "PAD\n")
                    buffers
                    events)
               (cl-letf
                   (((symbol-function 'get-buffer)
                     (lambda (name)
                       (push (list 'get-buffer name) events)
                       nil))
                    ((symbol-function 'switch-to-buffer-other-window)
                     (lambda (name)
                       (push (list 'switch name) events)
                       (set-buffer (get-buffer-create " *abc-mode-notes-test*"))))
                    ((symbol-function 'enlarge-window)
                     (lambda (amount)
                       (push (list 'enlarge amount) events)))
                    ((symbol-function 'window-height)
                     (lambda ()
                       (push '(window-height) events)
                       4))
                    ((symbol-function 'local-set-key)
                     (lambda (key command)
                       (push (list 'local-set-key key command) events)))
                    ((symbol-function 'other-window)
                     (lambda (count)
                       (push (list 'other-window count) events))))
                 (unwind-protect
                     (progn
                       (abc-mouse)
                       (setq buffers
                             (with-current-buffer
                                 " *abc-mode-notes-test*"
                               (list
                                (buffer-string)
                                (point))))
                       (list
                        buffers
                        (nreverse events)))
                   (when (get-buffer " *abc-mode-notes-test*")
                     (kill-buffer " *abc-mode-notes-test*")))))"##;
    let expect = expect![[
        r#"OK (("PAD\n" 1) ((get-buffer "*abc-notes*") (switch "*abc-notes*") (window-height) (enlarge -2) (local-set-key [mouse-1] abc-insert-mouse-note-other) (other-window 1)))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_align_bars_normalizes_internal_spaces_and_calls_aligner_exactly() {
    let elisp_form = r##"(let ((align-text-modes '(text-mode))
                    events)
               (with-temp-buffer
                 (insert "A   |   B\nC | D\nE|F\n")
                 (cl-letf
                     (((symbol-function 'align-regexp)
                       (lambda (&rest arguments)
                         (push arguments events)
                         'aligned)))
                   (let ((result
                          (abc-align-bars
                           (point-min)
                           (point-max))))
                     (list
                      result
                      (buffer-string)
                      align-text-modes
                      (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (aligned "A | B\nC | D\nE|F\n" (abc-mode text-mode) ((1 21 "\\(.\\)|" 1 1 t)))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_insert_chord_preserves_prompt_history_return_and_exact_quotes() {
    let elisp_form = r##"(let ((abc-chord-history '("Old"))
                    events)
               (with-temp-buffer
                 (insert "before ")
                 (cl-letf
                     (((symbol-function 'read-from-minibuffer)
                       (lambda (&rest arguments)
                         (push (cons 'read arguments) events)
                         "F♯m7")))
                   (list
                    (abc-insert-chord)
                    (buffer-string)
                    abc-chord-history
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (nil "before \"F♯m7\"" ("F♯m7" "Old") ((read "Chord: " nil nil nil abc-chord-history nil)))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_customize_forwards_the_exact_group_and_return_value() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'customize-group)
                     (lambda (group)
                       (push group events)
                       'customized)))
                 (list
                  (abc-customize)
                  (nreverse events))))"##;
    let expect = expect!["OK (customized (abc-mode))"];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_extract_chords_rewrites_notes_removes_decorations_and_preserves_other_lines() {
    let elisp_form = r##"(with-temp-buffer
               (insert "prefix\n\"Am\" ^C,D=e_f'g (a) [Bc] z2 |: [1 X !trill! % comment\nsuffix")
               (goto-char 15)
               (let ((kill-ring nil))
                 (abc-extract-chords)
                 (list
                  (buffer-string)
                  (point)
                  (point-min)
                  (point-max)
                  kill-ring)))"##;
    let expect = expect![[
        r#"OK ("prefix\n\"Am\" xxxxx x xx z2 |: 1 X !trill! % xommxnt\nsuffix" 51 1 58 ("e" "c" "c" "B" "a" "g" "f" "e" "D" "C"))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_midi_chords_skeleton_inserts_the_complete_custom_chord_table() {
    let elisp_form = r##"(let ((abc-midi-chord-list "CHORD-A\nCHORD-B"))
               (with-temp-buffer
                 (insert "before|after")
                 (goto-char 7)
                 (list
                  (abc-midi-chords)
                  (buffer-string)
                  (point))))"##;
    let expect = expect![[r#"OK (t "beforeCHORD-A\nCHORD-B\n|after" 22)"#]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_song_skeletons_forward_their_exact_declarative_programs() {
    let elisp_form = r##"(let (events)
               (cl-letf
                   (((symbol-function 'skeleton-proxy-new)
                     (lambda (skeleton str arg)
                       (push
                        (list skeleton str arg)
                        events)
                       'skeleton-result)))
                 (list
                  (abc-staves "wrapped" '(4))
                  (abc-skeleton nil -1)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (skeleton-result skeleton-result ((("" n abc-staves-tag " {" ("Staff #: " " " str) & " }" | -11) "wrapped" (4)) ((nil abc-reference-tag (if (abc-current-song-number t) (number-to-string (1+ (abc-current-song-number t))) "1") ("Title: " n abc-title-tag str) n abc-filename-tag "$Id: abc-mode.el 906 2013-01-11 15:52:26Z junker $" ("Lyricist: " n abc-lyricist-tag str) ("Composer: " n abc-composer-tag str) ("Book (original source): " n abc-book-tag str) ("Source (current source): " n abc-source-tag str) ("Notes (alternate sources, copyright): " n abc-notes-tag str) n abc-meter-tag (skeleton-read "Meter: " abc-default-meter nil) n abc-length-tag (skeleton-read "Default length: " abc-default-length nil) n abc-tempo-tag (skeleton-read "Tempo: " abc-default-tempo nil) '(abc-staves) n abc-key-tag (skeleton-read "Key: " abc-default-key nil) "\n%%MIDI program " '(unless (abc-insert-instrument "Main") (beginning-of-line) (kill-line) (backward-char)) "\n%%MIDI bassprog " '(unless (abc-insert-instrument "Bass") (beginning-of-line) (kill-line) (backward-char)) "\n%%MIDI chordprog " '(unless (abc-insert-instrument "Chord") (beginning-of-line) (kill-line) (backward-char)) n _) nil -1)))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

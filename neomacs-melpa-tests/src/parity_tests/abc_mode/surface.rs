use expect_test::expect;

use super::assert_abc_mode_parity;

#[test]
fn abc_mode_public_surface_and_command_classification_match_the_pin() {
    let elisp_form = r##"(list
               (featurep 'abc-mode)
               (mapcar
                #'fboundp
                '(abc-set-abc2ps-option-set
                  abc-current-song-number
                  abc-renumber-songs
                  abc-crescendo-region
                  abc-diminuendo-region
                  abc-repeat-region
                  abc-slur-region
                  abc-list-buffer-songs
                  abc-run-abc2ps-base
                  abc-run-abc2ps-all
                  abc-run-abc2ps-one
                  abc-set-preprocess-options
                  abc-preprocess
                  abc-preprocess-buffer
                  abc-run-abc2midi
                  abc-run-abc2midi-one
                  abc-run-abc2abc
                  abc-forward-song
                  abc-backward-song
                  abc-mouse
                  abc-insert-mouse-note-other
                  abc-insert-note-string
                  abc-cursor-to-note
                  abc-event-to-note
                  abc-mode
                  abc-insert-instrument
                  abc-staves
                  abc-midi-chords
                  abc-skeleton
                  abc-align-bars
                  abc-insert-chord
                  abc-customize
                  abc-extract-chords))
               (mapcar
                #'commandp
                '(abc-set-abc2ps-option-set
                  abc-current-song-number
                  abc-run-abc2ps-base
                  abc-run-abc2ps-all
                  abc-preprocess
                  abc-mode
                  abc-staves
                  abc-midi-chords
                  abc-skeleton
                  abc-extract-chords))
               (mapcar
                (lambda (feature)
                  (featurep feature))
                '(easymenu newcomment autoinsert align cus-face)))"##;
    let expect = expect![
        "OK (t (t t t t t t t t t t t t t t t t t t t t t t t t t t t t t t t t t) (t t nil t t t t t t t) (t t t t t))"
    ];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_mode_defaults_options_tags_and_histories_match_the_pin() {
    let elisp_form = r##"(list
               abc-mode-hook
               abc-mode-comment-start
               abc-default-meter
               abc-default-length
               abc-default-tempo
               abc-default-key
               abc-song-number-regexp
               abc-use-song-as-page-delimiter
               abc-executable
               abc-preferred-options
               abc-option-alist
               abc-additional-options
               abc-midi-executable
               abc-abc2abc-executable
               abc-pp-executable
               abc-pp-options
               abc-pp-midi-macro
               (mapcar
                #'symbol-value
                '(abc-title-regexp
                  abc-reference-tag
                  abc-title-tag
                  abc-composer-tag
                  abc-lyricist-tag
                  abc-meter-tag
                  abc-length-tag
                  abc-tempo-tag
                  abc-parts-tag
                  abc-staves-tag
                  abc-key-tag
                  abc-area-tag
                  abc-book-tag
                  abc-discography-tag
                  abc-filename-tag
                  abc-group-tag
                  abc-history-tag
                  abc-information-tag
                  abc-notes-tag
                  abc-origin-tag
                  abc-rhythm-tag
                  abc-source-tag
                  abc-user-tag
                  abc-words-end-tag
                  abc-words-tag
                  abc-transcription-tag
                  abc-version-tag
                  abc-copyright-tag
                  abc-creator-tag
                  abc-charset-tag
                  abc-include-tag
                  abc-edited-by-tag))
               (list
                abc-option-history
                abc-additional-option-history
                abc-instrument-history
                abc-chord-history))"##;
    let expect = expect![[
        r#"OK (nil "%" "4/4" "1/4" "1/4=120" "C" "^[ \11]*X[ \11]*:[ \11]*\\([0-9]+\\)" t "abcm2ps" "" (("pretty" . "-p") ("pretty2" . "-P") ("fbook" . "-F fbook.fmt") ("landscape" . "-F landscape.fmt") ("tight" . "-F tight.fmt") ("none" . "")) "" "abc2midi" "abc2abc" nil "" "-MIDI" ("^T:" "X:" "T:" "C:" "A:" "M:" "L:" "Q:" "P:" "%%staves" "K:" "A:" "B:" "D:" "F:" "G:" "H:" "I:" "N:" "O:" "R:" "S:" "U:" "W:" "w:" "Z:" "%%abc-version " "%%abc-copyright " "%%abc-creator " "%%abc-charset " "%%abc-include " "%%abc-edited-by ") (nil nil nil nil))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_mode_midi_instrument_and_chord_tables_preserve_exact_boundary_data() {
    let elisp_form = r##"(list
               (length abc-midi-instruments-alist)
               (car abc-midi-instruments-alist)
               (nth 63 abc-midi-instruments-alist)
               (nth 127 abc-midi-instruments-alist)
               (cdr (assoc "Acoustic Grand Piano" abc-midi-instruments-alist))
               (cdr (assoc "Flute" abc-midi-instruments-alist))
               (cdr (assoc "Gunshot" abc-midi-instruments-alist))
               (cdr (assoc "Piano" abc-midi-instruments-alist))
               (cdr (assoc "Violin" abc-midi-instruments-alist))
               (car (rassoc 0 abc-midi-instruments-alist))
               (car (rassoc 40 abc-midi-instruments-alist))
               (list
                (length abc-midi-chord-list)
                (substring abc-midi-chord-list 0 30)
                (substring abc-midi-chord-list -30))
               (list
                (length abc-midi-default-chord-list)
                (substring abc-midi-default-chord-list 0 30)
                (substring abc-midi-default-chord-list -30)))"##;
    let expect = expect![[
        r#"OK (165 ("Acoustic Grand Piano" . 0) ("SynthBrass 2" . 63) ("Gunshot" . 127) 0 73 127 nil 40 "Acoustic Grand Piano" "Violin" (413 "% additional\n%%MIDI chordname " "ordname 13-9   0 4 7 10 13 21\n") (795 "% Default chords\n%%MIDI chordn" "0\n%%MIDI chordname 5      0 7\n"))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_mode_registers_file_patterns_auto_insert_and_complete_keymap() {
    let elisp_form = r##"(list
               (cdr (assoc "\\.abc\\'" auto-mode-alist))
               (cdr (assoc "\\.abp\\'" auto-mode-alist))
               (cdr (assq 'abc-mode auto-insert-alist))
               (mapcar
                (lambda (key)
                  (lookup-key abc-mode-map (kbd key)))
                '("M-p"
                  "M-n"
                  "C-c C-l"
                  "C-c C-s"
                  "C-c C-n"
                  "C-c C-i"
                  "C-c C-d c"
                  "C-c C-d d"
                  "C-c C-d s"
                  "C-c C-m m"
                  "C-c C-m 1"
                  "C-c C-p o"
                  "C-c C-p p"
                  "C-c C-p 1"
                  "C-c C-a a"
                  "C-c C-a p"
                  "C-c C-a o"
                  "C-c C-c"))
               (mapcar
                (lambda (key)
                  (lookup-key abc-mode-old-map key))
                '("\M-p"
                  "\M-n"
                  "\C-c\C-t"
                  "\C-c\C-s"
                  "\C-c\C-p"
                  "\C-c\C-o"
                  "\C-c\C-n"
                  "\C-c\C-m"
                  "\C-c\C-k"
                  "\C-c\C-l"
                  "\C-c\C-i"
                  "\C-c\C-dc"
                  "\C-c\C-dd"
                  "\C-c\C-c"
                  "\C-c\C-a")))"##;
    let expect = expect![
        "OK (abc-mode abc-mode abc-skeleton (abc-backward-song abc-forward-song abc-list-buffer-songs abc-skeleton abc-renumber-songs abc-insert-instrument abc-crescendo-region abc-diminuendo-region abc-slur-region abc-run-abc2midi abc-run-abc2midi-one abc-set-abc2ps-option-set abc-run-abc2ps-all abc-run-abc2ps-one abc-run-abc2abc abc-preprocess-buffer abc-set-preprocess-options abc-insert-chord) (abc-backward-song abc-forward-song abc-run-abc2ps-one abc-skeleton abc-preprocess-buffer abc-set-abc2ps-option-set abc-renumber-songs abc-run-abc2midi abc-run-abc2midi-one abc-list-buffer-songs abc-insert-instrument abc-crescendo-region abc-diminuendo-region abc-run-abc2ps-all abc-run-abc2abc))"
    ];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_mode_syntax_font_lock_and_menu_shape_match_the_pin() {
    let elisp_form = r##"(list
               (with-syntax-table abc-mode-syntax-table
                 (char-syntax ?%))
               (with-syntax-table abc-mode-syntax-table
                 (char-syntax ?\n))
               (length abc-font-lock-keywords)
               (car abc-font-lock-keywords)
               (nth 7 abc-font-lock-keywords)
               (car (last abc-font-lock-keywords))
               (car abc-mode-menu)
               (mapcar
                (lambda (entry)
                  (cond
                   ((stringp entry) entry)
                   ((vectorp entry) (aref entry 0))
                   ((consp entry) (car entry))
                   (t entry)))
                (cdr abc-mode-menu)))"##;
    let expect = expect![[
        r#"OK (60 62 28 ("^[ \11]*[A-JL-SUX-Z][ \11]*:[^%\n]*" 0 'font-lock-keyword-face t) ("[_^]?[=_^][A-Ga-g]" quote font-lock-warning-face) ("^#redefine" 0 'font-lock-keyword-face t) keymap ("abc" Forward\ Song Backward\ Song List\ Songs nil New\ Song Renumber\ Songs nil-6 Dynamics Marks Bar\ Lines Slur\ Region Lyrics Fields MIDI nil-14 nil-15 Run\ abc2ps\ \(buffer\) Run\ abc2ps\ \(song\) Set\ abc2ps\ Options Select\ Option\ Set nil-20 nil-21 Run\ abc2midi\ \(buffer\) Run\ abc2midi\ \(song\) nil-24 Preprocess\ Buffer Set\ Preprocess\ Options... nil-27 Run\ abc2abc nil-29 Enable\ Mouse\ Input Remove\ Mouse\ Window))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_mode_initializes_all_buffer_local_state_and_runs_hook_last() {
    let elisp_form = r##"(let ((abc-mode-comment-start "%%")
                    (abc-song-number-regexp "^REF:[0-9]+")
                    (abc-use-song-as-page-delimiter t)
                    events)
               (with-temp-buffer
                 (setq-local sentence-end "ORIGINAL")
                 (let ((abc-mode-hook
                        (list
                         (lambda ()
                           (push
                            (list
                             major-mode
                             mode-name
                             comment-start
                             page-delimiter)
                            events)))))
                   (abc-mode)
                   (list
                    (nreverse events)
                    major-mode
                    mode-name
                    (derived-mode-p 'text-mode)
                    (eq (current-local-map) abc-mode-map)
                    (eq (syntax-table) abc-mode-syntax-table)
                    comment-start
                    comment-end
                    comment-start-skip
                    sentence-end
                    font-lock-defaults
                    (local-variable-p 'abc-options)
                    (local-variable-p 'page-delimiter)
                    page-delimiter))))"##;
    let expect = expect![[
        r#"OK (((abc-mode "abc" "%%" "^REF:[0-9]+") (abc-mode "abc" "%%" "^REF:[0-9]+")) abc-mode "abc" text-mode t t "%%" "" "\\(\\(^\\|[^\\\\\n]\\)\\(\\\\\\\\\\)*\\)%+ *" "|\\|" (abc-font-lock-keywords nil nil) t t "^REF:[0-9]+")"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_mode_leaves_page_delimiter_inherited_when_song_pages_are_disabled() {
    let elisp_form = r##"(let ((abc-use-song-as-page-delimiter nil))
               (with-temp-buffer
                 (let ((inherited page-delimiter))
                   (abc-mode)
                   (list
                    inherited
                    page-delimiter
                    (local-variable-p 'page-delimiter)))))"##;
    let expect = expect![[r#"OK ("^\f" "^\f" nil)"#]];

    assert_abc_mode_parity(elisp_form, expect);
}

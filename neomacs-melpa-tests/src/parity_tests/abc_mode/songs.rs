use expect_test::expect;

use super::{assert_abc_mode_parity, assert_abc_mode_signal_parity};

#[test]
fn abc_current_song_number_handles_boundaries_spacing_and_preserves_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert "preface\n X : 007\nT:One\nnotes\n\tX:\t42 tail\nT:Two\n")
               (list
                (progn
                  (goto-char (point-min))
                  (list (abc-current-song-number t) (point)))
                (progn
                  (search-forward "One")
                  (list (abc-current-song-number) (point)))
                (progn
                  (goto-char (point-max))
                  (list (abc-current-song-number) (point)))))"##;
    let expect = expect!["OK ((nil 1) (7 23) (42 48))"];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_current_song_number_signals_exactly_when_no_song_precedes_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert "preface only")
               (abc-current-song-number))"##;
    let expect = expect!["ERR (error \"Cannot find song number\")"];

    assert_abc_mode_signal_parity(elisp_form, expect);
}

#[test]
fn abc_renumber_songs_rewrites_only_matching_headers_and_preserves_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert "intro\n  X : 99 trailing\nT: First\nX:not-a-number\n\tX:\t3\nX: 0007 suffix\n")
               (goto-char 4)
               (let ((kill-ring nil))
                 (abc-renumber-songs)
                 (list
                  (buffer-string)
                  (point)
                  kill-ring)))"##;
    let expect = expect![[
        r#"OK ("intro\nX:1\nT: First\nX:not-a-number\nX:2\nX:3\n" 4 ("X: 0007 suffix" "\11X:\0113" "  X : 99 trailing"))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_region_commands_preserve_direction_and_exact_delimiters() {
    let elisp_form = r##"(mapcar
               (lambda (case)
                 (with-temp-buffer
                   (insert "left αβ right")
                   (goto-char (if (car case) 9 6))
                   (set-mark (if (car case) 6 9))
                   (let ((kill-ring nil))
                     (funcall (cdr case))
                     (list
                      (buffer-string)
                      (point)
                      (mark)
                      (car kill-ring)))))
               '((nil . abc-crescendo-region)
                 (t . abc-diminuendo-region)
                 (nil . abc-repeat-region)
                 (t . abc-slur-region)))"##;
    let expect = expect![[
        r#"OK (("left !crescendo(!αβ !crescendo)!right" 33 18 "αβ ") ("left !diminuendo(!αβ !diminuendo)!right" 35 19 "αβ ") ("left  |: αβ  :| right" 17 10 "αβ ") ("left (αβ )right" 11 7 "αβ "))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_song_navigation_returns_match_positions_and_stops_at_buffer_edges() {
    let elisp_form = r##"(with-temp-buffer
               (insert "X:1\nT:A\nbody\n  X : 22\nT:B\n")
               (goto-char (point-min))
               (list
                (abc-forward-song)
                (point)
                (abc-forward-song)
                (point)
                (abc-forward-song)
                (point)
                (abc-backward-song)
                (point)
                (abc-backward-song)
                (point)
                (abc-backward-song)
                (point)))"##;
    let expect = expect!["OK (4 4 22 22 nil 22 14 14 1 1 nil 1)"];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_list_buffer_songs_calls_occur_then_selects_the_exact_buffer() {
    let elisp_form = r##"(let ((abc-title-regexp "^TITLE:")
                    events)
               (cl-letf
                   (((symbol-function 'occur)
                     (lambda (regexp)
                       (push (list 'occur regexp) events)
                       'occur-result))
                    ((symbol-function 'pop-to-buffer)
                     (lambda (buffer)
                       (push (list 'pop buffer) events)
                       'pop-result)))
                 (list
                  (abc-list-buffer-songs)
                  (nreverse events))))"##;
    let expect = expect![[r#"OK (pop-result ((occur "^TITLE:") (pop "*Occur*")))"#]];

    assert_abc_mode_parity(elisp_form, expect);
}

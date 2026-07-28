use expect_test::expect;

use super::assert_ace_jump_mode_parity;

/// The headline workflow of the package: with `C-c SPC' bound as the README
/// suggests, `C-u C-c SPC' starts char mode, the query char paints one labelled
/// overlay per match on a greyed background, and the label key moves point
/// there in one keystroke.  The trace also pins the mode line, that the session
/// tears itself down (no overlays, no `overriding-local-map', no search tree)
/// and that the position the user came from is recorded on both mark rings.
#[test]
fn char_mode_labels_every_match_and_jumps_to_the_chosen_one() {
    let elisp_form = r##"(aj-test-with-live-buffer
 (insert aj-test-prose)
 (goto-char 50)
 (aj-test-tracing
  (execute-kbd-macro (kbd "C-u C-c SPC e c"))
  (list (point)
        (current-column)
        (line-number-at-pos)
        (buffer-substring-no-properties (point) (line-end-position))
        ace-jump-current-mode
        ace-jump-query-char
        ace-jump-mode
        overriding-local-map
        ace-jump-search-tree
        ace-jump-background-overlay-list
        (aj-test-mark-ring)
        (mark t)
        (buffer-substring-no-properties (point-min) (point-max)))))"##;
    let expect = expect![[
        r#"OK (((nil "" 50 nil nil nil) (nil "C-u" 50 nil nil nil) (ace-jump-mode "C-c SPC e" 50 ace-jump-char-mode " AceJump - Char" ((1 165 nil ace-jump-face-background) (3 4 "a" ace-jump-face-foreground) (29 30 "b" ace-jump-face-foreground) (34 35 "c" ace-jump-face-foreground) (66 67 "d" ace-jump-face-foreground) (71 72 "e" ace-jump-face-foreground) (92 93 "f" ace-jump-face-foreground) (112 113 "g" ace-jump-face-foreground) (127 128 "h" ace-jump-face-foreground) (143 144 "i" ace-jump-face-foreground) (150 151 "j" ace-jump-face-foreground) (162 163 "k" ace-jump-face-foreground))) (ace-jump-move "c" 34 nil nil nil)) 34 33 1 "e lazy dog." nil nil nil nil nil nil ((50 . "*ace-jump-workflow*")) 50 "The quick brown fox jumps over the lazy dog.\nPack my box with five dozen liquor jugs.\nHow vexingly quick daft zebras jump!\nQuiet quails quibble by the Quarry gate.\n")"#
    ]];

    assert_ace_jump_mode_parity(elisp_form, expect);
}

/// Word mode is the default submode, so a plain `C-c SPC' asks for a head char
/// and labels only word beginnings - the `q' inside "liquor" is never a
/// candidate.  Repeating the jump with `ace-jump-mode-case-fold' turned off
/// drops the capitalised "Quiet" and "Quarry", which renumbers the remaining
/// labels: the very same word is reached by `d' with folding and by `c'
/// without it.
#[test]
fn word_mode_marks_word_starts_and_honours_case_folding() {
    let elisp_form = r##"(aj-test-with-live-buffer
 (insert aj-test-prose)
 (goto-char (point-min))
 (list
  (aj-test-tracing
   (execute-kbd-macro (kbd "C-c SPC q d"))
   (list (point) (buffer-substring-no-properties (point) (+ (point) 6))))
  (progn
    (goto-char (point-min))
    (let ((ace-jump-mode-case-fold nil))
      (aj-test-tracing
       (execute-kbd-macro (kbd "C-c SPC q c"))
       (list (point) (buffer-substring-no-properties (point) (+ (point) 7))))))))"##;
    let expect = expect![[
        r#"OK ((((nil "" 1 nil nil nil) (ace-jump-mode "C-c SPC q" 1 ace-jump-word-mode " AceJump - Word" ((1 165 nil ace-jump-face-background) (5 6 "a" ace-jump-face-foreground) (100 101 "b" ace-jump-face-foreground) (124 125 "c" ace-jump-face-foreground) (130 131 "d" ace-jump-face-foreground) (137 138 "e" ace-jump-face-foreground) (152 153 "f" ace-jump-face-foreground))) (ace-jump-move "d" 130 nil nil nil)) 130 "quails") (((nil "" 1 nil nil nil) (ace-jump-mode "C-c SPC q" 1 ace-jump-word-mode " AceJump - Word" ((1 165 nil ace-jump-face-background) (5 6 "a" ace-jump-face-foreground) (100 101 "b" ace-jump-face-foreground) (130 131 "c" ace-jump-face-foreground) (137 138 "d" ace-jump-face-foreground))) (ace-jump-move "c" 130 nil nil nil)) 130 "quails "))"#
    ]];

    assert_ace_jump_mode_parity(elisp_form, expect);
}

/// Two prefix arguments select the third entry of
/// `ace-jump-mode-submode-list', line mode, which needs no query char: every
/// line beginning gets a label.  Jumping keeps the column the user was in, the
/// way `C-n' does, so starting at column 33 of line 1 lands on column 33 of
/// line 3 rather than on its first character.
#[test]
fn line_mode_marks_every_line_and_keeps_the_column() {
    let elisp_form = r##"(aj-test-with-live-buffer
 (insert aj-test-prose)
 (goto-char 34)
 (aj-test-tracing
  (execute-kbd-macro (kbd "C-u C-u C-c SPC c"))
  (list (point)
        (current-column)
        (line-number-at-pos)
        (buffer-substring-no-properties (line-beginning-position) (line-end-position))
        ace-jump-current-mode
        (aj-test-mark-ring))))"##;
    let expect = expect![[
        r#"OK (((nil "" 34 nil nil nil) (nil "C-u" 34 nil nil nil) (nil "C-u" 34 nil nil nil) (ace-jump-mode "C-c SPC" 34 ace-jump-line-mode " AceJump - Line" ((1 2 "a" ace-jump-face-foreground) (1 165 nil ace-jump-face-background) (46 47 "b" ace-jump-face-foreground) (87 88 "c" ace-jump-face-foreground) (124 125 "d" ace-jump-face-foreground))) (ace-jump-move "c" 120 nil nil nil)) 120 33 3 "How vexingly quick daft zebras jump!" nil ((34 . "*ace-jump-workflow*")))"#
    ]];

    assert_ace_jump_mode_parity(elisp_form, expect);
}

/// A user who cut `ace-jump-mode-move-keys' down to three keys gets the
/// N-branch search tree the package is built around: eleven matches share three
/// labels, the first key narrows to that branch and relabels its five members
/// while every other label disappears, and the second key jumps.  Only the
/// original position is pushed on the mark ring, once.
#[test]
fn custom_move_keys_build_a_two_level_search_tree() {
    let elisp_form = r##"(aj-test-with-live-buffer
 (insert aj-test-prose)
 (goto-char (point-min))
 (let ((ace-jump-mode-move-keys '(?a ?s ?d)))
   (aj-test-tracing
    (execute-kbd-macro (kbd "C-u C-c SPC e a s"))
    (list (point)
          (buffer-substring-no-properties (point) (+ (point) 8))
          ace-jump-current-mode
          (aj-test-mark-ring)))))"##;
    let expect = expect![[
        r#"OK (((nil "" 1 nil nil nil) (nil "C-u" 1 nil nil nil) (ace-jump-mode "C-c SPC e" 1 ace-jump-char-mode " AceJump - Char" ((1 165 nil ace-jump-face-background) (3 4 "a" ace-jump-face-foreground) (29 30 "a" ace-jump-face-foreground) (34 35 "a" ace-jump-face-foreground) (66 67 "a" ace-jump-face-foreground) (71 72 "a" ace-jump-face-foreground) (92 93 "s" ace-jump-face-foreground) (112 113 "s" ace-jump-face-foreground) (127 128 "s" ace-jump-face-foreground) (143 144 "d" ace-jump-face-foreground) (150 151 "d" ace-jump-face-foreground) (162 163 "d" ace-jump-face-foreground))) (ace-jump-move "a" 1 ace-jump-char-mode " AceJump - Char" ((1 165 nil ace-jump-face-background) (3 4 "a" ace-jump-face-foreground) (29 30 "a" ace-jump-face-foreground) (34 35 "a" ace-jump-face-foreground) (66 67 "s" ace-jump-face-foreground) (71 72 "d" ace-jump-face-foreground))) (ace-jump-move "s" 66 nil nil nil)) 66 "e dozen " nil ((1 . "*ace-jump-workflow*")))"#
    ]];

    assert_ace_jump_mode_parity(elisp_form, expect);
}

/// The other half of the README setup: `C-x SPC' walks back through
/// `ace-jump-mode-mark-ring'.  Two jumps record where each started, then two
/// pops return along the same route while the ring rotates instead of
/// shrinking, so the history can be walked round again.  Mark syncing is
/// enabled through the documented `ace-jump-mode-enable-mark-sync', which is
/// what puts the first jump's origin on the ordinary `mark-ring' as well.
#[test]
fn pop_mark_walks_back_through_the_ace_jump_mark_ring() {
    let elisp_form = r##"(aj-test-with-live-buffer
 (insert aj-test-prose)
 (unwind-protect
     (progn
       (ace-jump-mode-enable-mark-sync)
       (goto-char 50)
       (let ((ace-jump-mode-mark-ring nil)
             (mark-ring nil))
         (execute-kbd-macro (kbd "C-u C-c SPC e c"))
         (let ((first-jump (list (point) (aj-test-mark-ring) (mark t))))
           (execute-kbd-macro (kbd "C-u C-c SPC z a"))
           (let ((second-jump (list (point) (aj-test-mark-ring))))
             (execute-kbd-macro (kbd "C-x SPC"))
             (let ((first-pop (list (point) (aj-test-mark-ring))))
               (execute-kbd-macro (kbd "C-x SPC"))
               (list first-jump
                     second-jump
                     first-pop
                     (list (point) (aj-test-mark-ring))
                     ace-jump-sync-emacs-mark-ring
                     (mapcar #'marker-position mark-ring)))))))
   (ace-jump-mode-disable-mark-sync)))"##;
    let expect = expect![[
        r#"OK ((34 ((50 . "*ace-jump-workflow*")) 50) (38 ((34 . "*ace-jump-workflow*") (50 . "*ace-jump-workflow*"))) (34 ((50 . "*ace-jump-workflow*") (34 . "*ace-jump-workflow*"))) (50 ((34 . "*ace-jump-workflow*") (50 . "*ace-jump-workflow*"))) t (50))"#
    ]];

    assert_ace_jump_mode_parity(elisp_form, expect);
}

/// Aborting.  A key that carries no label ends the session through the `[t]'
/// binding, a label key past the end of the candidate list says so and ends it
/// too, and `C-g' typed at the labels is delivered as an ordinary key, so it
/// ends the session as well instead of terminating the macro.  In all three
/// cases point never moves, no overlay survives, and nothing is pushed onto the
/// mark ring.
#[test]
fn an_unassigned_key_ends_ace_jump_mode_and_leaves_point_alone() {
    let elisp_form = r##"(aj-test-with-live-buffer
 (insert aj-test-prose)
 (goto-char 50)
 (let ((ace-jump-mode-mark-ring nil))
   (list
    (aj-test-tracing
     (execute-kbd-macro (kbd "C-u C-c SPC e !"))
     (list (point) ace-jump-current-mode overriding-local-map (aj-test-mark-ring)))
    (aj-test-tracing
     (execute-kbd-macro (kbd "C-u C-c SPC e Z"))
     (list (point) ace-jump-current-mode (aj-test-last-message) (aj-test-mark-ring)))
    (condition-case error
        (progn (execute-kbd-macro (kbd "C-u C-c SPC e C-g")) :no-quit)
      (quit (list :quit (point) ace-jump-current-mode
                  (length (overlays-in (point-min) (point-max)))))
      (error (list :error error)))
    (list (point) ace-jump-current-mode overriding-local-map
          (length (overlays-in (point-min) (point-max)))
          (aj-test-mark-ring)))))"##;
    let expect = expect![[
        r#"OK ((((nil "" 50 nil nil nil) (nil "C-u" 50 nil nil nil) (ace-jump-mode "C-c SPC e" 50 ace-jump-char-mode " AceJump - Char" ((1 165 nil ace-jump-face-background) (3 4 "a" ace-jump-face-foreground) (29 30 "b" ace-jump-face-foreground) (34 35 "c" ace-jump-face-foreground) (66 67 "d" ace-jump-face-foreground) (71 72 "e" ace-jump-face-foreground) (92 93 "f" ace-jump-face-foreground) (112 113 "g" ace-jump-face-foreground) (127 128 "h" ace-jump-face-foreground) (143 144 "i" ace-jump-face-foreground) (150 151 "j" ace-jump-face-foreground) (162 163 "k" ace-jump-face-foreground))) (ace-jump-done "!" 50 nil nil nil)) 50 nil nil nil) (((nil "" 50 nil nil nil) (nil "C-u" 50 nil nil nil) (ace-jump-mode "C-c SPC e" 50 ace-jump-char-mode " AceJump - Char" ((1 165 nil ace-jump-face-background) (3 4 "a" ace-jump-face-foreground) (29 30 "b" ace-jump-face-foreground) (34 35 "c" ace-jump-face-foreground) (66 67 "d" ace-jump-face-foreground) (71 72 "e" ace-jump-face-foreground) (92 93 "f" ace-jump-face-foreground) (112 113 "g" ace-jump-face-foreground) (127 128 "h" ace-jump-face-foreground) (143 144 "i" ace-jump-face-foreground) (150 151 "j" ace-jump-face-foreground) (162 163 "k" ace-jump-face-foreground))) (ace-jump-move "Z" 50 nil nil nil)) 50 nil "No such position candidate." nil) :no-quit (50 nil nil 0 nil))"#
    ]];

    assert_ace_jump_mode_parity(elisp_form, expect);
}

/// Degenerate queries.  A punctuation head char is not a word constituent, so
/// word mode hands over to char mode (`ace-jump-mode-detect-punc'); that query
/// matches exactly once, so ace-jump skips the labels entirely, says so, and
/// jumps straight there while still recording the origin.  A query char that
/// occurs nowhere signals the package's own error and leaves everything alone,
/// and with punctuation detection disabled word mode refuses the head char.
#[test]
fn degenerate_queries_jump_directly_fall_back_or_report_no_match() {
    let elisp_form = r##"(aj-test-with-live-buffer
 (insert aj-test-prose)
 (goto-char 50)
 (let ((ace-jump-mode-mark-ring nil))
   (list
    (aj-test-tracing
     (execute-kbd-macro (kbd "C-c SPC !"))
     (list (point)
           (buffer-substring-no-properties (line-beginning-position) (point))
           ace-jump-current-mode
           (aj-test-last-message)
           (aj-test-mark-ring)))
    (let ((before (point)))
      (list (condition-case error
                (progn (execute-kbd-macro (kbd "C-u C-c SPC 7")) :no-error)
              (error error))
            (= before (point))
            ace-jump-current-mode
            (length (overlays-in (point-min) (point-max)))
            (aj-test-mark-ring)))
    (let ((ace-jump-mode-detect-punc nil))
      (condition-case error
          (progn (execute-kbd-macro (kbd "C-c SPC !")) :no-error)
        (error error))))))"##;
    let expect = expect![[
        r#"OK ((((nil "" 50 nil nil nil) (ace-jump-mode "C-c SPC !" 122 ace-jump-char-mode nil nil)) 122 "How vexingly quick daft zebras jump" ace-jump-char-mode "[AceJump] One candidate, move to it directly" ((50 . "*ace-jump-workflow*"))) ((error "[AceJump] No one found") t nil 0 nil) (error "[AceJump] Not a valid word constituent"))"#
    ]];

    assert_ace_jump_mode_parity(elisp_form, expect);
}

/// `ace-jump-mode-scope' decides how far a jump can reach.  With two windows on
/// screen, `window' scope labels only the selected window's matches, while the
/// default `global' scope continues the label sequence into the other window's
/// buffer - and choosing one of those labels selects that window, makes its
/// buffer current and moves point there.  Both buffers are left without a
/// single overlay.
#[test]
fn global_scope_labels_every_window_and_jumps_across_buffers() {
    let elisp_form = r##"(aj-test-with-live-buffer
 (insert aj-test-prose)
 (goto-char 50)
 (let ((notes (generate-new-buffer "*ace-jump-notes*"))
       (ace-jump-mode-before-jump-hook '(aj-test-capture-labels))
       (aj-test-labels nil))
   (unwind-protect
       (progn
         (with-current-buffer notes
           (insert "Zebra notes:\n  zeal, zenith, zephyr\n"))
         (set-window-buffer (split-window) notes)
         (list
          (length (window-list))
          (let ((ace-jump-mode-scope 'window))
            (execute-kbd-macro (kbd "C-u C-c SPC z b"))
            (list (buffer-name)
                  (point)
                  (buffer-name (window-buffer (selected-window)))
                  aj-test-labels
                  (aj-test-mark-ring)))
          (progn
            (setq aj-test-labels nil)
            (goto-char 50)
            (execute-kbd-macro (kbd "C-u C-c SPC z d"))
            (list (buffer-name)
                  (point)
                  (buffer-name (window-buffer (selected-window)))
                  (buffer-substring-no-properties
                   (line-beginning-position) (line-end-position))
                  aj-test-labels
                  (aj-test-mark-ring)))
          (mapcar (lambda (buffer)
                    (cons (buffer-name buffer)
                          (length (with-current-buffer buffer
                                    (overlays-in (point-min) (point-max))))))
                  (aj-test-workflow-buffers))))
     (kill-buffer notes))))"##;
    let expect = expect![[
        r#"OK (2 ("*ace-jump-workflow*" 70 "*ace-jump-workflow*" (("*ace-jump-notes*") ("*ace-jump-workflow*" (1 165 nil ace-jump-face-background) (38 39 "a" ace-jump-face-foreground) (70 71 "b" ace-jump-face-foreground) (111 112 "c" ace-jump-face-foreground))) ((50 . "*ace-jump-workflow*"))) ("*ace-jump-notes*" 1 "*ace-jump-notes*" "Zebra notes:" (("*ace-jump-notes*" (1 2 "d" ace-jump-face-foreground) (1 37 nil ace-jump-face-background) (16 17 "e" ace-jump-face-foreground) (22 23 "f" ace-jump-face-foreground) (30 31 "g" ace-jump-face-foreground)) ("*ace-jump-workflow*" (1 165 nil ace-jump-face-background) (38 39 "a" ace-jump-face-foreground) (70 71 "b" ace-jump-face-foreground) (111 112 "c" ace-jump-face-foreground))) ((50 . "*ace-jump-workflow*") (50 . "*ace-jump-workflow*"))) (("*ace-jump-notes*" . 0) ("*ace-jump-workflow*" . 0)))"#
    ]];

    assert_ace_jump_mode_parity(elisp_form, expect);
}

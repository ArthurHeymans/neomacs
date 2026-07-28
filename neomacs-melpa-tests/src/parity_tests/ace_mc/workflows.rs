use expect_test::expect;

use super::assert_ace_mc_parity;

/// The headline workflow with the line submode the docstring documents for two
/// `C-u` prefixes: press the labels of two more lines, then type once and watch
/// all three cursors insert.  `C-g` from `mc/keymap` concludes the session.
#[test]
fn line_mode_prefixes_every_selected_line_and_the_mc_keymap_ends_the_session() {
    let elisp_form = r##"(ace-mc-test-in-buffer
 "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n"
 (setq ace-mc-test-recorded-labels nil)
 (add-hook 'ace-jump-mode-before-jump-hook #'ace-mc-test-record-labels)
 (unwind-protect
     (progn
       (execute-kbd-macro (kbd "C-u C-u C-c m b c RET"))
       (let ((added (ace-mc-test-state)))
         (execute-kbd-macro (kbd "T O D O : SPC"))
         (let ((typed (ace-mc-test-state)))
           (execute-kbd-macro (kbd "C-g"))
           (list :labels (reverse ace-mc-test-recorded-labels)
                 :added added
                 :typed typed
                 :quit (ace-mc-test-state)
                 :keymap (list (lookup-key mc/keymap (kbd "C-g"))
                               (lookup-key mc/keymap (kbd "<return>")))))))
   (remove-hook 'ace-jump-mode-before-jump-hook #'ace-mc-test-record-labels)))"##;

    let expect = expect![[
        r#"OK (:labels (((1 . "a") (24 . "b") (41 . "c")) ((1 . "a") (24 . "b") (41 . "c"))) :added (:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 1 :cursors (24 41) :num 3 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil) :typed (:text "TODO: alpha beta alpha gamma\nTODO: alpha delta beta\nTODO: omega alpha stop\n" :point 7 :cursors (36 59) :num 3 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil) :quit (:text "TODO: alpha beta alpha gamma\nTODO: alpha delta beta\nTODO: omega alpha stop\n" :point 7 :cursors nil :num 1 :mc-mode nil :ace-mode nil :ace-marking nil :overriding nil) :keymap (mc/keyboard-quit multiple-cursors-mode))"#
    ]];

    assert_ace_mc_parity(elisp_form, expect);
}

/// "When the region is active, prompt for AceJump matches based on matching
/// strings": select the first `alpha`, label the other three occurrences, then
/// rename all four at once with `M-d` and typing.  No query character is read
/// on this path.
#[test]
fn an_active_region_adds_cursors_at_every_occurrence_and_renames_them_all() {
    let elisp_form = r##"(ace-mc-test-in-buffer
 "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n"
 (transient-mark-mode 1)
 (goto-char (point-min))
 (push-mark (point) nil t)
 (forward-word)
 (let ((region (list (region-active-p) (mark) (point)
                     (buffer-substring-no-properties (mark) (point)))))
   (execute-kbd-macro (kbd "C-c m b c d RET"))
   (let ((added (ace-mc-test-state))
         (mode (list (region-active-p) ace-mc-ace-mode-function)))
     (execute-kbd-macro (kbd "M-d"))
     (let ((killed (ace-mc-test-state)))
       (execute-kbd-macro (kbd "d e l t a"))
       (list :region region :mode mode :added added
             :killed killed :renamed (ace-mc-test-state))))))"##;

    let expect = expect![[
        r#"OK (:region (t 1 6 "alpha") :mode (nil ace-mc-regexp-mode) :added (:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 1 :cursors (12 24 47) :num 4 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil) :killed (:text " beta  gamma\n delta beta\nomega  stop\n" :point 1 :cursors (7 14 32) :num 4 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil) :renamed (:text "delta beta delta gamma\ndelta delta beta\nomega delta stop\n" :point 6 :cursors (17 29 52) :num 4 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil))"#
    ]];

    assert_ace_mc_parity(elisp_form, expect);
}

/// "If a cursor already exists at that location, it will be removed.  This
/// makes it easy to remove cursors you've added accidentally."  Selecting the
/// same label a second time takes the cursor away again, and removing the last
/// one leaves multiple-cursors-mode off.
#[test]
fn selecting_the_same_label_again_removes_that_cursor_and_the_last_one_ends_the_mode() {
    let elisp_form = r##"(ace-mc-test-in-buffer
 "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n"
 (execute-kbd-macro (kbd "C-u C-u C-c m b c RET"))
 (let ((added (ace-mc-test-state)))
   (execute-kbd-macro (kbd "C-u C-u C-c m b RET"))
   (let ((removed-one (ace-mc-test-state)))
     (execute-kbd-macro (kbd "C-u C-u C-c m c RET"))
     (list :added added
           :removed-one removed-one
           :removed-all (ace-mc-test-state)))))"##;

    let expect = expect![[
        r#"OK (:added (:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 1 :cursors (24 41) :num 3 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil) :removed-one (:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 1 :cursors (41) :num 2 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil) :removed-all (:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 1 :cursors nil :num 1 :mc-mode nil :ace-mode nil :ace-marking nil :overriding nil))"#
    ]];

    assert_ace_mc_parity(elisp_form, expect);
}

/// `ace-mc-add-single-cursor` is "a non-looping version": one label ends the
/// session, so the very next key is typed into the buffer instead of being
/// taken as another ace-jump label -- `Z` is a move key, so it would have been
/// swallowed had the loop still been running.
#[test]
fn the_single_cursor_command_stops_after_one_selection() {
    let elisp_form = r##"(ace-mc-test-in-buffer
 "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n"
 (execute-kbd-macro (kbd "C-u C-u C-c s b"))
 (let ((single (ace-mc-test-state))
       (looping ace-mc-loop-marking))
   (execute-kbd-macro (kbd "Z"))
   (list :single single :loop-marking looping :typed (ace-mc-test-state))))"##;

    let expect = expect![[
        r#"OK (:single (:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 1 :cursors (24) :num 2 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil) :loop-marking nil :typed (:text "Zalpha beta alpha gamma\nZalpha delta beta\nomega alpha stop\n" :point 2 :cursors (26) :num 2 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil))"#
    ]];

    assert_ace_mc_parity(elisp_form, expect);
}

/// Leaving the loop, as the docstring puts it, "by pressing return or escape".
/// Quitting immediately adds nothing and leaves point where it was; quitting
/// after one selection keeps the cursor that was already placed.  Line mode
/// keeps the starting column, so the cursor lands at column 7 of line 2.
#[test]
fn aborting_leaves_point_alone_and_keeps_cursors_added_before_the_abort() {
    let elisp_form = r##"(ace-mc-test-in-buffer
 "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n"
 (goto-char 8)
 (execute-kbd-macro (kbd "C-u C-u C-c m RET"))
 (let ((immediate (ace-mc-test-state)))
   (execute-kbd-macro (kbd "C-u C-u C-c m b C-g"))
   (list :immediate immediate
         :column (list (current-column) (save-excursion (goto-char 31) (current-column)))
         :aborted (ace-mc-test-state))))"##;

    let expect = expect![[
        r#"OK (:immediate (:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 8 :cursors nil :num 1 :mc-mode nil :ace-mode nil :ace-marking nil :overriding nil) :column (7 7) :aborted (:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 8 :cursors (31) :num 2 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil))"#
    ]];

    assert_ace_mc_parity(elisp_form, expect);
}

/// The two submodes that read a query character first: the default prefix picks
/// `ace-jump-word-mode`, which labels word starts beginning with the typed
/// character, and one `C-u` picks `ace-jump-char-mode`, which labels every
/// occurrence of it.  Cursors from both rounds then type together.
#[test]
fn the_query_char_submodes_add_cursors_at_word_starts_and_at_characters() {
    let elisp_form = r##"(ace-mc-test-in-buffer
 "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n"
 (execute-kbd-macro (kbd "C-c m a b c RET"))
 (let ((word (list (ace-mc-test-state) ace-mc-ace-mode-function ace-mc-query-char)))
   (execute-kbd-macro (kbd "C-u C-c m b a RET"))
   (let ((char (list (ace-mc-test-state) ace-mc-ace-mode-function ace-mc-query-char)))
     (execute-kbd-macro (kbd "!"))
     (list :word word :char char :typed (ace-mc-test-state)))))"##;

    let expect = expect![[
        r#"OK (:word ((:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 1 :cursors (12 24) :num 3 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil) ace-jump-word-mode 97) :char ((:text "alpha beta alpha gamma\nalpha delta beta\nomega alpha stop\n" :point 1 :cursors (7 12 24) :num 4 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil) ace-jump-char-mode 98) :typed (:text "!alpha !beta !alpha gamma\n!alpha delta beta\nomega alpha stop\n" :point 2 :cursors (9 15 28) :num 4 :mc-mode t :ace-mode nil :ace-marking nil :overriding nil))"#
    ]];

    assert_ace_mc_parity(elisp_form, expect);
}

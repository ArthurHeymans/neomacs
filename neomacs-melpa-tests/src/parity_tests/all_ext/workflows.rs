use expect_test::expect;

use super::assert_all_ext_parity;

/// `all' collects every matching line into `*All*': a header naming the regexp
/// and the source, then one piece per match.  Each piece is an overlay holding
/// a marker into the source buffer - those markers are what makes the buffer
/// editable - alongside the left-margin line numbers and the `match' face on
/// the matched text.  The source is left untouched by the collection itself.
#[test]
fn collecting_matches_builds_a_linked_all_buffer() {
    let elisp_form = r##"(ae-test-with-source
 (all "alpha")
 (list (with-current-buffer "*All*"
         (list major-mode
               (buffer-name all-buffer)
               next-error-function
               buffer-read-only
               (point)))
       (ae-test-text "*All*")
       (ae-test-pieces)
       (ae-test-line-numbers)
       (ae-test-match-faces)
       (ae-test-text source)))"##;
    let expect = expect![[
        r#"OK ((all-mode "notes.txt" all-next-error nil 1) "Lines matching \"alpha\" in buffer notes.txt.\n--------\nalpha one\nalpha four\nalpha six\n" ((54 64 "alpha one\n" 1) (64 75 "alpha four\n" 32) (75 85 "alpha six\n" 54)) ((54 . "1") (64 . "4") (75 . "6")) ((54 59 "alpha") (64 69 "alpha") (75 80 "alpha")) "alpha one\nbeta two\ngamma three\nalpha four\ndelta five\nalpha six\n")"#
    ]];

    assert_all_ext_parity(elisp_form, expect);
}

/// The package's whole point.  Replacing the text of one collected line, and
/// then inserting at the start of another, changes those lines in the *source*
/// buffer as the edit happens - and the pieces' markers follow, so the second
/// edit lands at the position the first one shifted it to.
#[test]
fn editing_a_collected_line_writes_back_to_the_source() {
    let elisp_form = r##"(ae-test-with-source
 (all "alpha")
 (with-current-buffer "*All*"
   (goto-char (point-min))
   (search-forward "alpha four")
   (replace-match "ALPHA FOUR!")
   (let ((after-first (list (ae-test-text "*All*") (ae-test-text source))))
     (goto-char (point-min))
     (search-forward "alpha six")
     (goto-char (line-beginning-position))
     (insert "TODO ")
     (list after-first
           (ae-test-text "*All*")
           (ae-test-text source)
           (with-current-buffer source (list (point) (buffer-modified-p)))
           (ae-test-pieces)))))"##;
    let expect = expect![[
        r#"OK (("Lines matching \"alpha\" in buffer notes.txt.\n--------\nalpha one\nALPHA FOUR!\nalpha six\n" "alpha one\nbeta two\ngamma three\nALPHA FOUR!\ndelta five\nalpha six\n") "Lines matching \"alpha\" in buffer notes.txt.\n--------\nalpha one\nALPHA FOUR!\nTODO alpha six\n" "alpha one\nbeta two\ngamma three\nALPHA FOUR!\ndelta five\nTODO alpha six\n" (1 t) ((54 64 "alpha one\n" 1) (64 76 "ALPHA FOUR!\n" 32) (76 91 "TODO alpha six\n" 55)))"#
    ]];

    assert_all_ext_parity(elisp_form, expect);
}

/// Deleting a collected line's text empties that line in the source, leaving
/// the line itself - the piece survives as an empty one, still linked - and
/// appending to another piece duplicates the word in the source too.  Both
/// edits are ordinary buffer commands in `*All*'.
#[test]
fn deleting_and_extending_collected_text_reaches_the_source() {
    let elisp_form = r##"(ae-test-with-source
 (all "alpha")
 (let ((deleted
        (with-current-buffer "*All*"
          (goto-char (point-min))
          (search-forward "alpha four")
          (delete-region (line-beginning-position) (line-end-position))
          (list (ae-test-text "*All*") (ae-test-text source)))))
   (with-current-buffer "*All*"
     (goto-char (point-min))
     (search-forward "alpha six")
     (insert " six")
     (list deleted
           (ae-test-text "*All*")
           (ae-test-text source)
           (ae-test-pieces)))))"##;
    let expect = expect![[
        r#"OK (("Lines matching \"alpha\" in buffer notes.txt.\n--------\nalpha one\n\nalpha six\n" "alpha one\nbeta two\ngamma three\n\ndelta five\nalpha six\n") "Lines matching \"alpha\" in buffer notes.txt.\n--------\nalpha one\n\nalpha six six\n" "alpha one\nbeta two\ngamma three\n\ndelta five\nalpha six six\n" ((54 64 "alpha one\n" 1) (64 65 "\n" 32) (65 79 "alpha six six\n" 44)))"#
    ]];

    assert_all_ext_parity(elisp_form, expect);
}

/// A deletion that starts inside one collected line and ends inside another has
/// no single source position to propagate to, so all.el refuses it before the
/// change happens: both buffers are exactly as they were, and an ordinary edit
/// inside one piece still works afterwards.
#[test]
fn an_edit_spanning_two_matches_is_refused() {
    let elisp_form = r##"(ae-test-with-source
 (all "alpha")
 (with-current-buffer "*All*"
   (goto-char (point-min))
   (let* ((start (progn (search-forward "alpha one") (- (point) 3)))
          (end (progn (search-forward "alpha four") (- (point) 4))))
     (let ((refused (condition-case error (progn (delete-region start end) :deleted)
                      (error error))))
       (goto-char (point-min))
       (search-forward "alpha one")
       (insert "!")
       (list refused
             (ae-test-text "*All*")
             (ae-test-text source)
             (with-current-buffer source (buffer-modified-p)))))))"##;
    let expect = expect![[
        r#"OK ((error "Changes should be limited to a single text piece") "Lines matching \"alpha\" in buffer notes.txt.\n--------\nalpha one!\nalpha four\nalpha six\n" "alpha one!\nbeta two\ngamma three\nalpha four\ndelta five\nalpha six\n" t)"#
    ]];

    assert_all_ext_parity(elisp_form, expect);
}

/// Navigation.  `C-c C-c' on a collected line pops to the source buffer at the
/// matching position, and `next-error' - which all-ext installs by advising
/// `all-mode' - steps through the collection from the top.  `C-x h' marks the
/// collected lines without the header, and the mode's four keys are the ones
/// the two packages bind, including the multiple-cursors entry point that is
/// defined even though the package is not installed.
#[test]
fn the_all_buffer_navigates_back_to_the_source() {
    let elisp_form = r##"(ae-test-with-source
 (all "alpha")
 (set-window-buffer (selected-window) "*All*")
 (with-current-buffer "*All*"
   (goto-char (point-min))
   (search-forward "alpha four")
   (goto-char (line-beginning-position))
   (let ((keys (list (key-binding (kbd "C-c C-c"))
                     (key-binding (kbd "C-x h"))
                     (key-binding (kbd "C-c C-k"))
                     (key-binding (kbd "C-c C-m")))))
     (execute-kbd-macro (kbd "C-c C-c"))
     (let ((jumped (list (ae-test-copy (buffer-name (current-buffer)))
                         (ae-test-copy (buffer-name (window-buffer (selected-window))))
                         (with-current-buffer source
                           (list (point) (line-number-at-pos)
                                 (copy-sequence
                                  (buffer-substring-no-properties
                                   (line-beginning-position) (line-end-position))))))))
       (set-window-buffer (selected-window) "*All*")
       (with-current-buffer "*All*" (goto-char (point-min)))
       (let ((stepped (list (condition-case error (progn (next-error) :moved)
                              (error error))
                            (with-current-buffer source
                              (list (point) (line-number-at-pos)))
                            (with-current-buffer "*All*" (point)))))
         (set-window-buffer (selected-window) "*All*")
         (with-current-buffer "*All*"
           (execute-kbd-macro (kbd "C-x h"))
           (list keys jumped stepped
                 (list (point) (mark t) mark-active
                       (copy-sequence
                        (buffer-substring-no-properties
                         (min (point) (mark t)) (max (point) (mark t))))))))))))"##;
    let expect = expect![[
        r#"OK ((all-mode-goto all-mark-whole-contents quit-window mc/edit-lines-in-all) ("notes.txt" "notes.txt" (32 4 "alpha four")) (:moved (1 1) 54) (54 85 t "alpha one\nalpha four\nalpha six\n"))"#
    ]];

    assert_all_ext_parity(elisp_form, expect);
}

/// With a context argument each match brings its neighbouring lines, and
/// because the three matches are close together their ranges overlap and are
/// emitted as a single piece covering the whole file - with a separator after
/// it, every line numbered, and still only the matched words faced.
#[test]
fn context_lines_merge_overlapping_matches_into_one_piece() {
    let elisp_form = r##"(ae-test-with-source
 (all "alpha" 1)
 (list (ae-test-text "*All*")
       (ae-test-pieces)
       (ae-test-line-numbers)
       (ae-test-match-faces)))"##;
    let expect = expect![[
        r#"OK ("Lines matching \"alpha\" in buffer notes.txt.\n--------\nalpha one\nbeta two\ngamma three\nalpha four\ndelta five\nalpha six\n--------\n" ((54 117 "alpha one\nbeta two\ngamma three\nalpha four\ndelta five\nalpha six\n" 1)) ((54 . "1") (64 . "2") (73 . "3") (85 . "4") (96 . "5") (107 . "6")) ((54 59 "alpha") (85 90 "alpha") (107 112 "alpha")))"#
    ]];

    assert_all_ext_parity(elisp_form, expect);
}

/// A defect a user meets immediately: `all' begins by killing `*All*'
/// unconditionally, and `kill-buffer' signals when no buffer of that name
/// exists.  So the command fails in a fresh session, fails again on a second
/// try because nothing was created, and only works once an `*All*' buffer
/// happens to exist.
#[test]
fn the_first_invocation_fails_because_it_kills_a_buffer_that_is_not_there() {
    let elisp_form = r##"(let ((source (generate-new-buffer "notes.txt")))
  (unwind-protect
      (with-current-buffer source
        (insert ae-test-notes)
        (goto-char (point-min))
        (list (get-buffer "*All*")
              (condition-case error (progn (all "alpha") :collected) (error error))
              (get-buffer "*All*")
              (condition-case error (progn (all "alpha") :collected) (error error))
              (progn (get-buffer-create "*All*")
                     (condition-case error (progn (all "alpha") :collected) (error error)))
              (ae-test-text "*All*")))
    (when (get-buffer "*All*") (kill-buffer "*All*"))
    (kill-buffer source)))"##;
    let expect = expect![[
        r#"OK (nil (error "No buffer named *All*") nil (error "No buffer named *All*") :collected "Lines matching \"alpha\" in buffer notes.txt.\n--------\nalpha one\nalpha four\nalpha six\n")"#
    ]];

    assert_all_ext_parity(elisp_form, expect);
}

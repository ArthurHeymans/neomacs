use expect_test::expect;

use super::assert_ado_mode_parity;

/// Opening any of the file kinds Stata uses has to land in this mode with the
/// right editing setup.  The package registers twenty `auto-mode-alist' entries
/// -- ten extensions in both cases -- and this visits one file of each kind: an
/// ado program, a do-file, a mata file, the two help formats, a label file, a
/// class, an SMCL log, a dialog and an internal help file, plus an upper-case
/// name and one extension the package does not claim.  Each buffer pins the
/// extension the mode deduced and whether smart indentation stayed on, because
/// the mode deliberately turns it off for the help and dialog formats.  The ado
/// buffer then pins the comment setup, the indenter, the tab width and the rest
/// of the contract.
#[test]
fn visiting_every_stata_file_kind_sets_up_the_editing_contract() {
    let elisp_form = r##"(progn
  (ado-test-configure)
  (let ((observed nil))
    (dolist (name '("mysum.ado" "analysis.do" "matrix.mata" "mysum.sthlp"
                    "old.hlp" "labels.lbl" "widget.class" "log.smcl"
                    "dialog.dlg" "internal.ihlp" "ANALYSIS.DO" "notes.txt"))
      (let ((buffer (ado-test-visit name "program define demo\nend\n")))
        (push (list name major-mode ado-extension
                    (and (boundp 'ado-smart-indent-flag) ado-smart-indent-flag))
              observed)
        (kill-buffer buffer)))
    (ado-test-visit "mysum.ado" ado-test-program)
    (list :routing (nreverse observed)
          :registered (length (seq-filter (lambda (entry) (eq (cdr entry) 'ado-mode))
                                          auto-mode-alist))
          :mode major-mode
          :mode-name mode-name
          :comment (list comment-start comment-end comment-column comment-start-skip
                         comment-multi-line)
          :indent (list indent-line-function tab-width ado-tab-width)
          :final-newline require-final-newline
          :parse-sexp-ignore-comments parse-sexp-ignore-comments
          :keymap (and (keymapp ado-mode-map) t))))"##;

    let expect = expect![[
        r##"OK (:routing (("mysum.ado" ado-mode "ado" t) ("analysis.do" ado-mode "do" t) ("matrix.mata" ado-mode "mata" t) ("mysum.sthlp" ado-mode "sthlp" nil) ("old.hlp" ado-mode "hlp" nil) ("labels.lbl" ado-mode "lbl" t) ("widget.class" ado-mode "class" t) ("log.smcl" ado-mode "smcl" t) ("dialog.dlg" ado-mode "dlg" nil) ("internal.ihlp" ado-mode "ihlp" t) ("ANALYSIS.DO" ado-mode "do" t) ("notes.txt" text-mode nil t)) :registered 20 :mode ado-mode :mode-name "Ado" :comment ("//" "" 40 "/\\*+ *" nil) :indent (ado-indent-line 3 3) :final-newline t :parse-sexp-ignore-comments t :keymap t)"##
    ]];

    assert_ado_mode_parity(elisp_form, expect);
}

/// Reading a realistic ado program: a version banner, `program define', a
/// `syntax' line, macro references, an `if'/`else' block and returned results.
/// The complete run-by-run face map pins the package's own face vocabulary --
/// commands that change data marked harmful and ones that do not marked
/// harmless, subcommands, local macros and `r()' results broken into function,
/// parentheses and name, strings and the `*!' version comment -- and that
/// visiting neither modifies the buffer nor moves point.
#[test]
fn font_lock_marks_commands_macros_results_strings_and_comments() {
    let elisp_form = r##"(progn
  (ado-test-visit "mysum.ado" ado-test-program)
  (list :runs (ado-test-face-runs)
        :modified (buffer-modified-p)
        :point (point)))"##;

    let expect = expect![[
        r##"OK (:runs (("*! version 1.0.0  01jan2020" . ado-comment-face) ("\n") ("program" . ado-builtin-harmful-face) (" ") ("define" . ado-subcommand-face) (" ") ("mysum" . ado-builtin-harmful-face) (", rclass\n\11") ("version" . ado-builtin-harmless-face) (" ") ("16" . ado-subcommand-face) ("\n\11") ("syntax" . ado-builtin-harmless-face) (" varlist(numeric) [if] [in] [, Detail]\n\11") ("marksample" . ado-builtin-harmful-face) (" ") ("touse" . ado-variable-name-face) ("\n\11") ("quietly" . ado-builtin-harmless-face) (" ") ("summarize" . ado-builtin-harmless-face) (" ") ("`varlist'" . ado-variable-name-face) (" ") ("if" . ado-mata-keyword-face) (" ") ("`touse'" . ado-variable-name-face) (", ") ("`detail'" . ado-variable-name-face) ("\n\11") ("if" . ado-builtin-harmless-face) (" `") ("r" . ado-function-name-face) ("(" . ado-constant-face) ("N" . ado-variable-name-face) (")" . ado-constant-face) ("' == 0 {\n\11\11") ("display" . ado-builtin-harmless-face) (" ") ("as" . ado-subcommand-face) (" ") ("error" . ado-subcommand-face) (" ") ("\"no observations\"" . font-lock-string-face) ("\n\11\11") ("exit" . ado-builtin-harmless-face) (" 2000\n\11}\n\11") ("else" . ado-builtin-harmless-face) (" {\n\11\11") ("display" . ado-builtin-harmless-face) (" ") ("as" . ado-subcommand-face) (" ") ("text" . ado-subcommand-face) (" ") ("\"mean = \"" . font-lock-string-face) (" as result %9.4f `") ("r" . ado-function-name-face) ("(" . ado-constant-face) ("mean" . ado-variable-name-face) (")" . ado-constant-face) ("'\n\11}\n\11") ("return" . ado-builtin-harmless-face) (" ") ("scalar" . ado-subcommand-face) (" mean = `") ("r" . ado-function-name-face) ("(" . ado-constant-face) ("mean" . ado-variable-name-face) (")" . ado-constant-face) ("'\n") ("end" . ado-builtin-harmful-face) ("\n")) :modified nil :point 1)"##
    ]];

    assert_ado_mode_parity(elisp_form, expect);
}

/// Reindenting a program someone pasted in flush left.  The mode's indenter
/// follows Stata's brace blocks and indents with the tab width the mode sets,
/// and `ado-find-depth' reports the nesting it used.  The same file is indented
/// twice, once fontified and once not, to pin that the indenter reads syntax
/// rather than faces and produces byte-identical output either way.
#[test]
fn reindenting_a_program_follows_the_brace_depth_without_fontification() {
    let elisp_form = r##"(progn
  (ado-test-visit "flat.ado"
    "program define mysum, rclass\nversion 16\nif `r(N)' == 0 {\ndisplay as error \"none\"\nexit 2000\n}\nelse {\ndisplay \"ok\"\n}\nend\n")
  (let ((before (ado-test-text)))
    (ado-indent-buffer)
    (let ((plain (ado-test-text)))
      (ado-test-visit "flat-fontified.ado"
        "program define mysum, rclass\nversion 16\nif `r(N)' == 0 {\ndisplay as error \"none\"\nexit 2000\n}\nelse {\ndisplay \"ok\"\n}\nend\n")
      (font-lock-ensure)
      (ado-indent-buffer)
      (list :before before
            :indented plain
            :same-when-fontified (equal plain (ado-test-text))
            :depth-at-display (save-excursion
                                (goto-char (point-min))
                                (search-forward "display as error")
                                (ado-find-depth))))))"##;

    let expect = expect![[
        r##"OK (:before "program define mysum, rclass\nversion 16\nif `r(N)' == 0 {\ndisplay as error \"none\"\nexit 2000\n}\nelse {\ndisplay \"ok\"\n}\nend\n" :indented "program define mysum, rclass\nversion 16\n\11if `r(N)' == 0 {\n\11\11display as error \"none\"\n\11\11exit 2000\n\11\11}\n\11else {\n\11\11display \"ok\"\n\11\11}\nend\n" :same-when-fontified t :depth-at-display (2 nil))"##
    ]];

    assert_ado_mode_parity(elisp_form, expect);
}

/// Stata commands run across lines with `///', so "the command at point" is not
/// "the line at point".  From the middle of a three-line continued `regress',
/// `ado-beginning-of-command' must reach the start of the `regress' and
/// `ado-end-of-command' the end of its last continuation, and copying the
/// command must yield all three lines.  This also pins that the two ways of
/// copying differ from the docstring's claim: the flag argument *returns* the
/// text, and calling with no argument is what fills the kill ring.
#[test]
fn command_motion_and_copying_span_slash_slash_slash_continuations() {
    let elisp_form = r##"(progn
  (ado-test-visit "motion.do"
    "sysuse auto, clear\nregress price mpg weight ///\n    if foreign == 0, ///\n    robust\nsummarize price\n")
  (goto-char (point-min))
  (forward-line 2)
  (end-of-line)
  (let* ((start (progn (ado-beginning-of-command)
                       (list (line-number-at-pos) (current-column))))
         (end (progn (ado-end-of-command)
                     (list (line-number-at-pos) (current-column))))
         (returned (ado-copy-command t)))
    (ado-copy-command)
    (list :beginning start
          :end end
          :returned returned
          :kill (substring-no-properties (current-kill 0))
          :delimiter-is-semi (ado-delimit-is-semi-p)
          :buffer-unchanged (equal (ado-test-text)
                                   "sysuse auto, clear\nregress price mpg weight ///\n    if foreign == 0, ///\n    robust\nsummarize price\n"))))"##;

    let expect = expect![[
        r##"OK (:beginning (2 0) :end (4 10) :returned "regress price mpg weight ///\n    if foreign == 0, ///\n    robust" :kill "regress price mpg weight ///\n    if foreign == 0, ///\n    robust" :delimiter-is-semi nil :buffer-unchanged t)"##
    ]];

    assert_ado_mode_parity(elisp_form, expect);
}

/// Commenting a block of a do-file out and putting it back.  The mode sets `//'
/// as the comment starter with an empty ender, so `comment-region' prefixes each
/// line and `uncomment-region' restores the file byte for byte.  A `//' comment
/// is fontified with the package's own comment face, and -- worth pinning --
/// `syntax-ppss' does not consider it a comment, because the mode paints `//'
/// through font-lock rather than through its syntax table.
#[test]
fn commenting_a_block_round_trips_and_is_painted_not_parsed() {
    let elisp_form = r##"(progn
  (ado-test-visit "comment.do" "sysuse auto, clear\nregress price mpg\nsummarize price\n")
  (goto-char (point-min))
  (let* ((beg (line-beginning-position))
         (end (progn (forward-line 2) (line-end-position)))
         (commented (progn (comment-region beg end) (ado-test-text))))
    (uncomment-region beg (line-end-position))
    (list :commented commented
          :restored (ado-test-text)
          :in-comment (save-excursion
                        (goto-char (point-min))
                        (insert "// a note about price\n")
                        (font-lock-ensure)
                        (goto-char (point-min))
                        (search-forward "note")
                        (list (nth 4 (syntax-ppss))
                              (get-text-property (1- (point)) 'face))))))"##;

    let expect = expect![[
        r##"OK (:commented "// sysuse auto, clear\n// regress price mpg\n// summarize price\n" :restored "sysuse auto, clear\nregress price mpg\nsummarize price\n" :in-comment (nil ado-comment-face))"##
    ]];

    assert_ado_mode_parity(elisp_form, expect);
}

/// `ado-macify-selection-or-word' wraps text in Stata's local-macro quotes, and
/// it has two branches: the active region, or the word at point when there is
/// none.  Both are pinned here, and deliberately so -- `transient-mark-mode' is
/// nil in batch, so the command silently takes its word-at-point path unless
/// the mode is turned on and a region really is active.  The workflow records
/// that default, exercises the word path under it, then enables the mode, marks
/// two words and pins the region path.
#[test]
fn macro_quoting_uses_the_region_only_when_one_is_really_active() {
    let elisp_form = r##"(progn
  (ado-test-visit "macify.do" "regress price mpg weight\n")
  (goto-char (point-min))
  (search-forward "price")
  (goto-char (match-beginning 0))
  (let ((default-mode transient-mark-mode))
    (ado-macify-selection-or-word)
    (let ((word-only (list (ado-test-text) (point))))
      (transient-mark-mode 1)
      (goto-char (point-min))
      (search-forward "mpg weight")
      (set-mark (match-beginning 0))
      (goto-char (match-end 0))
      (let ((region-state (list transient-mark-mode mark-active (use-region-p))))
        (ado-macify-selection-or-word)
        (list :transient-mark-mode-by-default default-mode
              :word-at-point word-only
              :region-state region-state
              :with-region (list (ado-test-text) (point)))))))"##;

    let expect = expect![[
        r##"OK (:transient-mark-mode-by-default nil :word-at-point ("regress `price' mpg weight\n" 16) :region-state (t t t) :with-region ("regress `price' `mpg weight'\n" 29))"##
    ]];

    assert_ado_mode_parity(elisp_form, expect);
}

/// Two editing conveniences.  imenu is opt-in: until the user runs
/// `ado-set-imenu-items' the buffer has no generic expression at all, and
/// afterwards the index lists each `program define' by name at its position,
/// with case folding turned off because Stata is case sensitive.  And
/// `ado-foreach-loop' writes a loop skeleton at point over the list it is given,
/// leaving point inside the body.
#[test]
fn imenu_is_opt_in_and_foreach_writes_a_loop_skeleton() {
    let elisp_form = r##"(progn
  (ado-test-visit "imenu.ado" ado-test-program)
  (let ((before imenu-generic-expression))
    (ado-set-imenu-items)
    (let ((case-fold imenu-case-fold-search)
          (after (mapcar (lambda (entry)
                           (cons (car entry)
                                 (if (markerp (cdr entry))
                                     (marker-position (cdr entry))
                                   (cdr entry))))
                         (funcall imenu-create-index-function))))
      (ado-test-visit "loop.do" "sysuse auto, clear\n")
      (goto-char (point-max))
      (ado-foreach-loop "v" "varlist price mpg")
      (list :generic-expression-before before
            :imenu-after after
            :case-fold case-fold
            :foreach (ado-test-text)
            :point (point)))))"##;

    let expect = expect![[
        r##"OK (:generic-expression-before nil :imenu-after (("mysum" . 29)) :case-fold nil :foreach "sysuse auto, clear\nforeach v of varlist price mpg   {\n\n\11}" :point 51)"##
    ]];

    assert_ado_mode_parity(elisp_form, expect);
}

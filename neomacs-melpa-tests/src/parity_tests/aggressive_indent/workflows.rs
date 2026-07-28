use expect_test::expect;

use super::assert_aggressive_indent_parity;

/// Wrapping existing code in a new form, by typing it: with point at the start
/// of `(message', the user types `(when request' and RET.  While the new form
/// is still open the defun is unbalanced and nothing below moves -- the package
/// will not reindent code it cannot parse -- and the moment the closing paren
/// is typed the whole defun is reindented and `(process request)' lands inside
/// the `when'.  That second step is the thing `electric-indent-mode' cannot do:
/// it is a line the user is not typing on.
#[test]
fn typing_a_wrapper_form_reindents_the_lines_it_encloses_once_it_is_balanced() {
    let elisp_form = r##"(agi-test-with-buffer
 'emacs-lisp-mode agi-test-lisp-defun
 (search-forward "(message")
 (goto-char (match-beginning 0))
 (execute-kbd-macro (kbd "( w h e n SPC r e q u e s t RET"))
 (let ((typed (agi-test-text)))
   (agi-test-idle)
   (let ((still-open (agi-test-state)))
     (goto-char (point-max))
     (search-backward "(process request))")
     (end-of-line)
     (execute-kbd-macro (kbd ")"))
     (agi-test-idle)
     (list :typed typed
           :while-unbalanced still-open
           :after-closing (agi-test-state)))))"##;
    let expect = expect![[
        r#"OK (:typed "(defun handler (request)\n  (when request\n    (message \"start\")\n  (process request))\n" :while-unbalanced (:text "(defun handler (request)\n  (when request\n    (message \"start\")\n  (process request))\n" :point 46 :line 3 :column 4 :mode t :electric t) :after-closing (:text "(defun handler (request)\n  (when request\n    (message \"start\")\n    (process request)))\n" :point 87 :line 4 :column 23 :mode t :electric t))"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}

/// The other half of the story: removing a wrapper.  Killing the `(when
/// request' line with two `C-k's leaves its former body indented one level too
/// deep, and once the user pauses, both surviving lines are dedented.  Point
/// stays where the user left it.
#[test]
fn deleting_the_enclosing_form_dedents_the_lines_it_contained() {
    let elisp_form = r##"(agi-test-with-buffer
 'emacs-lisp-mode agi-test-nested-lisp-defun
 (search-forward "(when request")
 (beginning-of-line)
 (execute-kbd-macro (kbd "C-k C-k"))
 (let ((killed (agi-test-state)))
   (agi-test-idle)
   (list :after-killing killed
         :after-idle (agi-test-state))))"##;
    let expect = expect![[
        r#"OK (:after-killing (:text "(defun handler (request)\n    (message \"start\")\n    (process request)))\n" :point 26 :line 2 :column 0 :mode t :electric t) :after-idle (:text "(defun handler (request)\n  (message \"start\")\n  (process request)))\n" :point 26 :line 2 :column 0 :mode t :electric t))"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}

/// The same behaviour in a C buffer, where indentation comes from cc-mode
/// rather than from lisp forms: opening an `if (ready) {' block above two
/// existing statements pulls the first one in as the user types, the second
/// follows when the editor goes idle, and typing the closing brace puts it at
/// the block's own level.
#[test]
fn opening_a_block_in_a_c_buffer_reindents_the_statements_it_swallows() {
    let elisp_form = r##"(agi-test-with-buffer
 'c-mode agi-test-c-function
 (search-forward "log(")
 (beginning-of-line)
 (execute-kbd-macro (kbd "i f SPC ( r e a d y ) SPC { RET"))
 (let ((typed (agi-test-text)))
   (agi-test-idle)
   (let ((opened (agi-test-state)))
     (goto-char (point-max))
     (search-backward "process(ready);")
     (end-of-line)
     (execute-kbd-macro (kbd "RET }"))
     (agi-test-idle)
     (list :typed typed
           :after-opening opened
           :after-closing (agi-test-state)))))"##;
    let expect = expect![[
        r#"OK (:typed "int handler(int ready) {\n  if (ready) {\n    log(\"start\");\n  process(ready);\n}\n" :after-opening (:text "int handler(int ready) {\n  if (ready) {\n    log(\"start\");\n    process(ready);\n}\n" :point 45 :line 3 :column 4 :mode t :electric t) :after-closing (:text "int handler(int ready) {\n  if (ready) {\n    log(\"start\");\n    process(ready);\n  }\n}\n" :point 82 :line 5 :column 3 :mode t :electric t))"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}

/// The mode rebinds BACKSPACE through a `menu-item' filter: pressed with point
/// sitting after a line's leading indentation it joins the line onto the
/// previous one (`delete-indentation') instead of nibbling one space at a time,
/// which is what you want when the indentation is not yours to edit.  Pressed
/// at the very beginning of the line the filter declines and the ordinary
/// backspace deletes the newline, keeping the indentation as it was.
#[test]
fn backspace_on_the_leading_indentation_joins_the_line_instead_of_deleting_a_space() {
    let elisp_form = r##"(list
 :after-indentation
 (agi-test-with-buffer
  'emacs-lisp-mode "(defun f ()\n  (message \"x\"))\n"
  (search-forward "(message")
  (goto-char (match-beginning 0))
  (let ((binding (key-binding [backspace])))
    (execute-kbd-macro (vector 'backspace))
    (let ((joined (agi-test-text)))
      (agi-test-idle)
      (list :binding binding :joined joined :after-idle (agi-test-text)))))
 :at-beginning-of-line
 (agi-test-with-buffer
  'emacs-lisp-mode "(defun f ()\n  (message \"x\"))\n"
  (search-forward "(message")
  (beginning-of-line)
  (let ((binding (key-binding [backspace])))
    (execute-kbd-macro (vector 'backspace))
    (let ((deleted (agi-test-text)))
      (agi-test-idle)
      (list :binding binding :deleted deleted :after-idle (agi-test-text))))))"##;
    let expect = expect![[
        r#"OK (:after-indentation (:binding delete-indentation :joined "(defun f () (message \"x\"))\n" :after-idle "(defun f () (message \"x\"))\n") :at-beginning-of-line (:binding nil :deleted "(defun f ()  (message \"x\"))\n" :after-idle "(defun f ()  (message \"x\"))\n"))"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}

/// The two ways a user tells it to keep quiet.  A form in
/// `aggressive-indent-dont-indent-if' that evaluates non-nil suppresses the
/// reindentation entirely, and `aggressive-indent-protected-commands' stops it
/// from running right after `undo' -- without which every undo would be fought
/// by a reindent.  Both halves run the identical edit, so the difference is
/// only the policy.
#[test]
fn the_dont_indent_if_and_protected_commands_policies_keep_it_quiet() {
    let elisp_form = r##"(list
 :dont-indent-if
 (agi-test-with-buffer
  'emacs-lisp-mode agi-test-nested-lisp-defun
  (let ((aggressive-indent-dont-indent-if '((looking-at-p "[[:space:]]*(message"))))
    (search-forward "(when request")
    (beginning-of-line)
    (execute-kbd-macro (kbd "C-k C-k"))
    (agi-test-idle)
    (agi-test-text)))
 :without-that-guard
 (agi-test-with-buffer
  'emacs-lisp-mode agi-test-nested-lisp-defun
  (search-forward "(when request")
  (beginning-of-line)
  (execute-kbd-macro (kbd "C-k C-k"))
  (agi-test-idle)
  (agi-test-text))
 :protected-after-undo
 (agi-test-with-buffer
  'emacs-lisp-mode agi-test-nested-lisp-defun
  (search-forward "(when request")
  (beginning-of-line)
  (execute-kbd-macro (kbd "C-k C-k"))
  (agi-test-idle)
  (execute-kbd-macro (kbd "C-/"))
  (agi-test-idle)
  (list :last-command last-command
        :protected (memq 'undo aggressive-indent-protected-commands)
        :text (agi-test-text)))
 :unprotected-after-undo
 (agi-test-with-buffer
  'emacs-lisp-mode agi-test-nested-lisp-defun
  (let ((aggressive-indent-protected-commands nil))
    (search-forward "(when request")
    (beginning-of-line)
    (execute-kbd-macro (kbd "C-k C-k"))
    (agi-test-idle)
    (execute-kbd-macro (kbd "C-/"))
    (agi-test-idle)
    (list :last-command last-command
          :protected aggressive-indent-protected-commands
          :text (agi-test-text)))))"##;
    let expect = expect![[
        r#"OK (:dont-indent-if "(defun handler (request)\n    (message \"start\")\n    (process request)))\n" :without-that-guard "(defun handler (request)\n  (message \"start\")\n  (process request)))\n" :protected-after-undo (:last-command undo :protected (undo undo-tree-undo undo-tree-redo undo-tree-visualize undo-tree-visualize-undo undo-tree-visualize-redo whitespace-cleanup) :text "(defun handler (request)\n\n    (message \"start\")\n    (process request)))\n") :unprotected-after-undo (:last-command undo :protected nil :text "(defun handler (request)\n\n  (message \"start\")\n  (process request)))\n"))"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}

/// What one `undo' gives back after the mode has reindented behind you.  The
/// package deliberately tries not to litter the undo history: a single undo
/// takes back the kill *and* the reindentation that followed it, rather than
/// leaving the user to undo the machine's edit first and their own second.
#[test]
fn one_undo_takes_back_both_the_edit_and_the_reindentation() {
    let elisp_form = r##"(agi-test-with-buffer
 'emacs-lisp-mode agi-test-nested-lisp-defun
 (search-forward "(when request")
 (beginning-of-line)
 (execute-kbd-macro (kbd "C-k C-k"))
 (agi-test-idle)
 (let ((reindented (agi-test-text)))
   (execute-kbd-macro (kbd "C-/"))
   (agi-test-idle)
   (list :original agi-test-nested-lisp-defun
         :after-edit reindented
         :after-one-undo (agi-test-text)
         :point (point))))"##;
    let expect = expect![[
        r#"OK (:original "(defun handler (request)\n  (when request\n    (message \"start\")\n    (process request)))\n" :after-edit "(defun handler (request)\n  (message \"start\")\n  (process request)))\n" :after-one-undo "(defun handler (request)\n\n    (message \"start\")\n    (process request)))\n" :point 26)"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}

/// `global-aggressive-indent-mode' turns the mode on everywhere except the
/// modes in `aggressive-indent-excluded-modes' and the ones where indentation
/// is not a thing -- text-mode and fundamental-mode stay clear.  The
/// distinction the documentation makes is that the exclusion list applies to
/// the global mode only: asking for `aggressive-indent-mode' by hand in a
/// text-mode buffer still turns it on.
#[test]
fn the_global_mode_skips_excluded_modes_while_the_local_command_does_not() {
    let elisp_form = r##"(progn
  (global-aggressive-indent-mode 1)
  (let ((under-global
         (mapcar (lambda (mode)
                   (let ((buffer (generate-new-buffer "*agi-global*")))
                     (unwind-protect
                         (with-current-buffer buffer
                           (funcall mode)
                           (list mode
                                 aggressive-indent-mode
                                 (and (memq #'aggressive-indent--keep-track-of-changes
                                            after-change-functions)
                                      t)))
                       (kill-buffer buffer))))
                 '(emacs-lisp-mode c-mode text-mode fundamental-mode))))
    (global-aggressive-indent-mode -1)
    (list :excluded aggressive-indent-excluded-modes
          :under-global under-global
          :global-off (let ((buffer (generate-new-buffer "*agi-off*")))
                        (unwind-protect
                            (with-current-buffer buffer
                              (emacs-lisp-mode)
                              (list aggressive-indent-mode global-aggressive-indent-mode))
                          (kill-buffer buffer)))
          :local-in-excluded-mode
          (agi-test-with-buffer
           'text-mode "hello\n"
           (list aggressive-indent-mode
                 (and (memq #'aggressive-indent--keep-track-of-changes
                            after-change-functions)
                      t))))))"##;
    let expect = expect![
        "OK (:excluded (elm-mode haskell-mode inf-ruby-mode makefile-mode makefile-gmake-mode python-mode sql-interactive-mode text-mode yaml-mode) :under-global ((emacs-lisp-mode t t) (c-mode t t) (text-mode nil nil) (fundamental-mode nil nil)) :global-off (nil nil) :local-in-excluded-mode (t t))"
    ];

    assert_aggressive_indent_parity(elisp_form, expect);
}

/// Saving is the other moment the package indents: `before-save-hook' runs the
/// pending reindentation synchronously, so code pasted in unindented is written
/// to disk properly indented even if the user never pauses.  The buffer and the
/// bytes on disk have to agree.
#[test]
fn saving_the_buffer_indents_what_was_typed_before_writing_it_to_disk() {
    let elisp_form = r##"(let ((path (agi-test-sandbox-file "project/handler.el")))
  (agi-test-with-buffer
   'emacs-lisp-mode ""
   (setq buffer-file-name path)
   (insert "(defun f (x)\n(when x\n(message \"hi\")))\n")
   (goto-char (point-max))
   (let ((before (agi-test-text)))
     (save-buffer)
     (list :before-save before
           :after-save (agi-test-text)
           :on-disk (agi-test-file-contents path)
           :modified (buffer-modified-p)
           :hook (and (memq #'aggressive-indent--process-changed-list-and-indent
                            before-save-hook)
                      t)))))"##;
    let expect = expect![[
        r#"OK (:before-save "(defun f (x)\n(when x\n(message \"hi\")))\n" :after-save "(defun f (x)\n  (when x\n    (message \"hi\")))\n" :on-disk "(defun f (x)\n  (when x\n    (message \"hi\")))\n" :modified nil :hook t)"#
    ]];

    assert_aggressive_indent_parity(elisp_form, expect);
}

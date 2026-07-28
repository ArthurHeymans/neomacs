use expect_test::expect;

use super::assert_alt_codes_parity;

/// The package's whole purpose: hold Meta, type a Windows alt code on the
/// keypad, and get the character.  Four codes cover the ranges a user reaches
/// for -- an ASCII letter, a Latin-1 letter, a three digit code, and a
/// four digit code whose leading zero selects the Windows-1252 table -- and
/// each one leaves the pending code cleared for the next.  Code 32 is included
/// because the shipped table spells it "spc": alt-32 inserts those three
/// letters rather than a space, which is the data's own quirk and not a
/// rendering artefact.
#[test]
fn typing_an_alt_code_on_the_keypad_inserts_the_character_it_names() {
    let elisp_form = r##"(alt-codes-test-with-buffer
 (let ((max-lisp-eval-depth 12800))
   (list :ascii (alt-codes-test-enter "65")
         :latin1 (alt-codes-test-enter "225")
         :windows-1252 (alt-codes-test-enter "0128")
         :accented (alt-codes-test-enter "0193")
         :spelled-space (alt-codes-test-enter "32")
         :empty-entry (alt-codes-test-enter "189")
         :hook (alt-codes-test-hook))))"##;
    let expect = expect![[
        r#"OK (:ascii ("A" "") :latin1 ("ß" "") :windows-1252 ("€" "") :accented ("Á" "") :spelled-space ("spc" "") :empty-entry ("" "") :hook (t t t))"#
    ]];

    assert_alt_codes_parity(elisp_form, expect);
}

/// What a user actually meets on a current Emacs.  The lookup builds a `pcase'
/// of all 383 table entries with `eval', which needs about 6400
/// `max-lisp-eval-depth' to expand; at the default 1600 the first alt code of
/// the session signals `excessive-lisp-nesting' from inside
/// `pre-command-hook'.  The digits were accumulated and announced, nothing is
/// inserted, the pending code is left uncleared, and the editor removes the
/// failing hook -- so the mode is still on but no longer does anything, and
/// ordinary typing continues.  Both editors agree on all of that; see
/// DIVERGENCES for the part they do not agree on.
#[test]
fn the_first_lookup_of_a_session_fails_at_the_default_eval_depth() {
    let elisp_form = r##"(alt-codes-test-with-buffer
 (let ((mark (alt-codes-test-message-mark)))
   (apply #'alt-codes-test-type (alt-codes-test-code ?6 ?5))
   (let ((pending (copy-sequence alt-codes--code)))
     (alt-codes-test-type 'f5)
     (let ((after (list (copy-sequence (buffer-string))
                        (copy-sequence alt-codes--code)
                        (alt-codes-test-hook))))
       (alt-codes-test-type ?z)
       (list :depth max-lisp-eval-depth
             :table-entries (length alt-codes--list)
             :announced (alt-codes-test-messages-since mark "Alt Code")
             :pending pending
             :after-commit after
             :hook-error (alt-codes-test-messages-since mark "pre-command-hook")
             :typing-still-works (copy-sequence (buffer-string))
             :raising-the-limit-works
             (let ((max-lisp-eval-depth 12800))
               (alt-codes--get-symbol "65")))))))"##;
    let expect = expect![[
        r#"OK (:depth 1600 :table-entries 383 :announced ("[Alt Code]: 6" "[Alt Code]: 65") :pending "65" :after-commit ("" "65" (nil t t)) :hook-error ("Error in pre-command-hook (alt-codes--pre-command-hook): (excessive-lisp-nesting 1601)") :typing-still-works "z" :raising-the-limit-works "A")"#
    ]];

    assert_alt_codes_parity(elisp_form, expect);
}

/// A consequence of the key the package chose.  `M-<kp-6>' reaches the hook as
/// the symbol it needs, but the editor also translates the keypad digit
/// through `function-key-map' to plain `6', so `M-6' runs `digit-argument' and
/// the same keystrokes build a numeric prefix argument.  A user typing alt-12
/// and then pressing a key runs that key's command twelve times, which is why
/// the fixture clears the prefix between codes.
#[test]
fn the_keypad_digits_also_build_a_numeric_prefix_argument() {
    let elisp_form = r##"(alt-codes-test-with-buffer
 (setq prefix-arg nil current-prefix-arg nil)
 (execute-kbd-macro (vconcat (alt-codes-test-code ?1 ?2)))
 (list :pending (copy-sequence alt-codes--code)
       :prefix prefix-arg
       :keypad-translation (lookup-key function-key-map [kp-6])
       :meta-digit-command (key-binding (kbd "M-6"))
       :keypad-command (key-binding [M-kp-6])
       :commit-runs-that-many-times
       (let ((max-lisp-eval-depth 12800))
         (execute-kbd-macro (vconcat [?x]))
         (copy-sequence (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (:pending "12" :prefix 12 :keypad-translation [54] :meta-digit-command digit-argument :keypad-command nil :commit-runs-that-many-times "x")"#
    ]];

    assert_alt_codes_parity(elisp_form, expect);
}

/// The design's real constraint: the hook only acts when the key just pressed
/// is a symbol, so an ordinary letter never commits the pending code -- it is
/// typed into the buffer and the digits stay pending, waiting for a function
/// key, an arrow or anything else that arrives as a symbol.  That is why this
/// package needs a graphical frame, where Return arrives as `<return>'.
#[test]
fn only_a_symbol_event_commits_the_pending_code() {
    let elisp_form = r##"(alt-codes-test-with-buffer
 (let ((max-lisp-eval-depth 12800))
   (apply #'alt-codes-test-type (alt-codes-test-code ?6 ?5))
   (let ((pending (copy-sequence alt-codes--code)))
     (alt-codes-test-type ?x)
     (let ((after-letter (list (copy-sequence (buffer-string))
                               (copy-sequence alt-codes--code))))
       (alt-codes-test-type 'f5)
       (list :pending pending
             :after-a-letter after-letter
             :after-a-symbol (list (copy-sequence (buffer-string))
                                   (copy-sequence alt-codes--code)))))))"##;
    let expect =
        expect![[r#"OK (:pending "65" :after-a-letter ("x" "65") :after-a-symbol ("xA" ""))"#]];

    assert_alt_codes_parity(elisp_form, expect);
}

/// Codes the table does not define insert nothing, and -- because the reset
/// sits outside the lookup -- the pending digits are cleared anyway, so the
/// next code starts from scratch rather than inheriting the failed one.  A
/// read-only buffer is left alone entirely: the hook returns before it even
/// looks at the key, so nothing accumulates.
#[test]
fn an_invalid_code_inserts_nothing_and_still_clears_the_pending_digits() {
    let elisp_form = r##"(alt-codes-test-with-buffer
 (let ((max-lisp-eval-depth 12800))
   (list :invalid (alt-codes-test-enter "9999")
         :next-code-is-unaffected (alt-codes-test-enter "65")
         :read-only
         (progn (erase-buffer)
                (setq buffer-read-only t)
                (apply #'alt-codes-test-type
                       (append (alt-codes-test-code ?6 ?5) (list 'f5)))
                (prog1 (list (copy-sequence (buffer-string))
                             (copy-sequence alt-codes--code))
                  (setq buffer-read-only nil))))))"##;
    let expect =
        expect![[r#"OK (:invalid ("" "") :next-code-is-unaffected ("A" "") :read-only ("" ""))"#]];

    assert_alt_codes_parity(elisp_form, expect);
}

/// Turning the mode on and off: it installs one buffer-local
/// `pre-command-hook' entry and removes exactly that entry, leaving the
/// editor's own entries in place.  With the mode off the same keystrokes are
/// ordinary input -- plain digits self-insert, and keypad digits accumulate
/// nothing -- and the globalized mode arms a new buffer the moment it gets a
/// major mode.
#[test]
fn the_mode_installs_and_removes_its_hook_and_leaves_plain_typing_alone() {
    let elisp_form = r##"(list
 :lifecycle
 (alt-codes-test-with-buffer
  (let ((on (alt-codes-test-hook)))
    (alt-codes-mode -1)
    (list :on on :off (alt-codes-test-hook)
          :hook-value (copy-sequence pre-command-hook))))
 :mode-off
 (let ((buffer (generate-new-buffer "*alt-codes-off*")))
   (unwind-protect
       (progn
         (set-window-buffer (selected-window) buffer)
         (set-buffer buffer)
         (text-mode)
         (setq prefix-arg nil current-prefix-arg nil)
         (local-set-key [f5] #'ignore)
         (execute-kbd-macro "65")
         (let ((typed (copy-sequence (buffer-string))))
           (erase-buffer)
           (setq prefix-arg nil current-prefix-arg nil)
           (execute-kbd-macro (vconcat (append (alt-codes-test-code ?6 ?5) (list 'f5))))
           (list :digits typed
                 :keypad (copy-sequence (buffer-string))
                 :hook (and (memq #'alt-codes--pre-command-hook pre-command-hook) t))))
     (kill-buffer buffer)))
 :globalized
 (progn
   (global-alt-codes-mode 1)
   (let ((armed (with-temp-buffer (text-mode) (alt-codes-test-hook))))
     (global-alt-codes-mode -1)
     (list :armed armed
           :after (with-temp-buffer (text-mode) (alt-codes-test-hook))))))"##;
    let expect = expect![[
        r#"OK (:lifecycle (:on (t t t) :off (nil t nil) :hook-value (eldoc-pre-command-refresh-echo-area t)) :mode-off (:digits "65" :keypad "" :hook nil) :globalized (:armed (t t t) :after (nil t nil)))"#
    ]];

    assert_alt_codes_parity(elisp_form, expect);
}

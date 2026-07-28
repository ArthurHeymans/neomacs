use expect_test::expect;

use super::assert_ac_slime_parity;

/// The installation the README prescribes: `set-up-slime-ac' on the
/// `slime-mode' and `slime-repl-mode' hooks, optionally with a prefix argument
/// for the fuzzy source.  This pins which source each call installs, that a
/// second call does not install it twice, that `ac-sources' stays buffer local
/// so a Lisp buffer and the REPL are configured independently, that
/// auto-complete already knows both modes, and -- because a source that is
/// installed but cannot complete is worthless -- that completing at the REPL
/// prompt really reaches swank and inserts its answer.
#[test]
fn set_up_slime_ac_installs_the_chosen_source_in_each_buffer_separately() {
    let elisp_form = r##"(progn
  (require 'slime)
  (acs-test-connect)
  (let ((lisp (acs-test-lisp-buffer "(defun demo ()\n  (ca"))
        (fuzzy (generate-new-buffer "*acs-fuzzy*"))
        (repl (get-buffer "*slime-repl sbcl*"))
        (observed nil))
    (push (list :modes-known (list (and (memq 'lisp-mode ac-modes) t)
                                   (and (memq 'slime-repl-mode ac-modes) t)))
          observed)
    (with-current-buffer lisp
      (push (list :lisp-before ac-sources) observed)
      (set-up-slime-ac)
      (set-up-slime-ac)
      (push (list :lisp-after ac-sources
                  :buffer-local (local-variable-p 'ac-sources))
            observed))
    (with-current-buffer fuzzy
      (set-up-slime-ac t)
      (push (list :fuzzy-buffer ac-sources) observed))
    (with-current-buffer repl
      (set-window-buffer (selected-window) repl)
      (set-up-slime-ac)
      (push (list :repl-mode major-mode :repl-sources ac-sources) observed)
      (goto-char (point-max))
      (insert "(str")
      (acs-test-complete)
      (push (list :repl-prefix ac-prefix
                  :repl-candidates (acs-test-candidates))
            observed)
      (ac-complete)
      (push (list :repl-line (acs-test-line)) observed))
    (with-current-buffer lisp
      (push (list :lisp-unchanged ac-sources) observed))
    (nreverse observed)))"##;

    let expect = expect![[
        r#"OK ((:modes-known (t t)) (:lisp-before #1=(ac-source-words-in-same-mode-buffers)) (:lisp-after #2=(ac-source-slime-simple . #1#) :buffer-local t) (:fuzzy-buffer (ac-source-slime-fuzzy . #1#)) (:repl-mode slime-repl-mode :repl-sources (ac-source-slime-simple . #1#)) (:repl-prefix "str" :repl-candidates ("string" "string=" "stringp")) (:repl-line "(string") (:lisp-unchanged #2#))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

/// The primary workflow: typing a prefix inside a form in a Lisp buffer and
/// completing it.  This pins the exact swank request the simple source issues
/// (including the quoted current package), the candidate list, the fact that
/// the prefix boundary comes from slime's own `slime-symbol-start-pos' rather
/// than auto-complete's word rules, the "l" marker every candidate carries,
/// that the simple source attaches no summary, and the buffer text and point
/// after insertion.
#[test]
fn completing_in_a_lisp_buffer_asks_swank_and_inserts_the_chosen_symbol() {
    let elisp_form = r##"(progn
  (require 'slime)
  (acs-test-connect)
  (acs-test-lisp-buffer "(defun demo ()\n  (ca")
  (set-up-slime-ac)
  (goto-char (point-max))
  (acs-test-complete)
  (let ((result (list :prefix ac-prefix
                      :prefix-start (slime-symbol-start-pos)
                      :candidates (acs-test-candidates)
                      :annotations (acs-test-summaries)
                      :requests (last (acs-test-swank-requests)))))
    (ac-complete)
    (append result (list :line (acs-test-line)
                         :point (point)
                         :mode major-mode))))"##;

    let expect = expect![[
        r#"OK (:prefix "ca" :prefix-start 19 :candidates ("car" "cadr" "case" "catch") :annotations (("car" nil "l") ("cadr" nil "l") ("case" nil "l") ("catch" nil "l")) :requests ("(:emacs-rex (swank:simple-completions \"ca\" '#1=\"COMMON-LISP-USER\") #1# t 4)") :line "  (car" :point 22 :mode lisp-mode)"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

/// The fuzzy source is a different request and a different presentation: it
/// asks `swank:fuzzy-completions' with the limit ac-slime binds and the time
/// limit slime configures, and it hangs swank's classification flags on each
/// candidate as its summary, which auto-complete shows beside the name.  With
/// `ac-slime-show-flags' turned off the same completion must offer the same
/// names with no summary at all.
#[test]
fn the_fuzzy_source_labels_each_candidate_with_the_flags_swank_returned() {
    let elisp_form = r##"(progn
  (require 'slime)
  (acs-test-connect)
  (acs-test-lisp-buffer "(defun demo ()\n  (ca")
  (set-up-slime-ac t)
  (goto-char (point-max))
  (acs-test-complete)
  (let ((observed (list (list :with-flags ac-slime-show-flags
                              :candidates (acs-test-candidates)
                              :annotations (acs-test-summaries)
                              :request (car (last (acs-test-swank-requests)))))))
    (setq ac-slime-show-flags nil)
    (acs-test-complete)
    (append observed
            (list (list :with-flags ac-slime-show-flags
                        :candidates (acs-test-candidates)
                        :annotations (acs-test-summaries))))))"##;

    let expect = expect![[
        r#"OK ((:with-flags t :candidates ("car" "cadr" "case" "catch") :annotations (("car" "-f--e-" "l") ("cadr" "-f--e-" "l") ("case" "-m----" "l") ("catch" "-m----" "l")) :request "(:emacs-rex (swank:fuzzy-completions \"ca\" #1=\"COMMON-LISP-USER\" :limit 50 :time-limit-in-msec 1500) #1# t 4)") (:with-flags nil :candidates ("car" "cadr" "case" "catch") :annotations (("car" nil "l") ("cadr" nil "l") ("case" nil "l") ("catch" nil "l"))))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

/// Common Lisp symbols are case insensitive, so swank answers an upper-case
/// prefix with its own lower-case symbol names.  The simple source's match
/// function exists to fix that up: every candidate keeps the capitalisation the
/// user actually typed, and completing inserts it that way.
#[test]
fn an_uppercase_prefix_is_carried_into_every_candidate_and_inserted() {
    let elisp_form = r##"(progn
  (require 'slime)
  (acs-test-connect)
  (acs-test-lisp-buffer "(defun demo ()\n  (CA")
  (set-up-slime-ac)
  (goto-char (point-max))
  (acs-test-complete)
  (let ((result (list :prefix ac-prefix
                      :candidates (acs-test-candidates)
                      :request (car (last (acs-test-swank-requests))))))
    (ac-complete)
    (append result (list :line (acs-test-line) :point (point)))))"##;

    let expect = expect![[
        r#"OK (:prefix "CA" :candidates ("CAr" "CAdr" "CAse" "CAtch") :request "(:emacs-rex (swank:simple-completions \"CA\" '#1=\"COMMON-LISP-USER\") #1# t 4)" :line "  (CAr" :point 22)"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

/// Every candidate carries a documentation function that auto-complete's popup
/// calls to fill the quick help.  Asking three of the offered candidates pins
/// that each is looked up separately in the running Lisp with its own symbol
/// name, that the answer is passed through unchanged, and that a symbol the
/// Lisp has no documentation for still produces the Lisp's own answer rather
/// than an error.
#[test]
fn each_candidate_documents_itself_from_the_running_lisp() {
    let elisp_form = r##"(progn
  (require 'slime)
  (acs-test-connect)
  (acs-test-lisp-buffer "(defun demo ()\n  (ca")
  (set-up-slime-ac)
  (goto-char (point-max))
  (acs-test-complete)
  (list :car (popup-item-documentation (nth 0 ac-candidates))
        :case (popup-item-documentation (nth 2 ac-candidates))
        :catch (popup-item-documentation (nth 3 ac-candidates))
        :requests (last (acs-test-swank-requests) 3)))"##;

    let expect = expect![[
        r#"OK (:car "Return the car of LIST.  Signals TYPE-ERROR otherwise." :case "CASE keyform {({key | (key*)} form*)}*" :catch "Not documented." :requests ("(:emacs-rex (swank:documentation-symbol \"car\") \"COMMON-LISP-USER\" t 5)" "(:emacs-rex (swank:documentation-symbol \"case\") \"COMMON-LISP-USER\" t 6)" "(:emacs-rex (swank:documentation-symbol \"catch\") \"COMMON-LISP-USER\" t 7)"))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

/// Editing Lisp before connecting is the normal state of affairs.  Both
/// sources are documented to be silent then: no candidate, and -- with a swank
/// server listening but no connection made -- not a single request may be sent
/// to it.  Completion itself must keep working from the buffer's own words.
#[test]
fn without_a_connection_neither_source_contacts_swank() {
    let elisp_form = r##"(progn
  (require 'slime)
  (acs-test-start-swank)
  (acs-test-lisp-buffer "(defun cabinet () nil)\n(caboose)\n(ca")
  (set-up-slime-ac)
  (goto-char (point-max))
  (acs-test-complete)
  (list :connected (slime-connected-p)
        :simple (ac-source-slime-simple-candidates)
        :fuzzy (ac-source-slime-fuzzy-candidates)
        :prefix ac-prefix
        :candidates (acs-test-candidates)
        :sources ac-sources
        :requests (acs-test-swank-requests)
        :line (acs-test-line)))"##;

    let expect = expect![[
        r#"OK (:connected nil :simple nil :fuzzy nil :prefix "ca" :candidates ("cabinet" "caboose") :sources (ac-source-slime-simple ac-source-words-in-same-mode-buffers) :requests nil :line "(ca")"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_ac_inf_ruby_parity;

/// The package's headline workflow, set up exactly as its commentary
/// prescribes: `inf-ruby-mode' added to `ac-modes', TAB rebound to
/// `auto-complete' in `inf-ruby-mode-map', and `ac-inf-ruby-enable' on the mode
/// hook -- called twice here, because a hook that runs again must not add the
/// source twice.  The user then types at the REPL prompt and completes from
/// what the live Ruby process reports, and a second, narrower prefix is asked
/// of the process again and is unique, so auto-complete's dwim expansion
/// inserts it straight away.
#[test]
fn ac_inf_ruby_completes_a_repl_expression_from_the_live_ruby_process() {
    let elisp_form = r##"(ac-inf-ruby-test-with-repl
 (setq ac-modes (cons 'inf-ruby-mode ac-modes))
 (define-key inf-ruby-mode-map (kbd "TAB") #'auto-complete)
 (setq-local ac-sources nil)
 (ac-inf-ruby-enable)
 (ac-inf-ruby-enable)
 (auto-complete-mode 1)
 (let ((installed (list :sources ac-sources
                        :buffer-local (local-variable-p 'ac-sources)
                        :source ac-source-inf-ruby
                        :in-ac-modes (and (memq 'inf-ruby-mode ac-modes) t)
                        :auto-complete auto-complete-mode
                        :tab (lookup-key inf-ruby-mode-map (kbd "TAB")))))
   (goto-char (point-max))
   (execute-kbd-macro (kbd "S t r TAB"))
   (let ((offered (list (ac-inf-ruby-test-session) (ac-inf-ruby-test-menu))))
     (execute-kbd-macro (kbd "M-n"))
     (let ((moved (ac-inf-ruby-test-session)))
       (execute-kbd-macro (kbd "RET"))
       (let ((completed (ac-inf-ruby-test-buffer-state)))
         (comint-send-input)
         (ac-inf-ruby-test-wait-for-prompt)
         (execute-kbd-macro (kbd "S t r u TAB"))
         (list :installed installed
               :offered offered
               :moved moved
               :completed completed
               :unique (ac-inf-ruby-test-buffer-state)
               :requests (ac-inf-ruby-test-requests)))))))"##;

    let expect = expect![[
        r#"OK (:installed (:sources #1=(ac-source-inf-ruby) :buffer-local t :source ((available . ac-inf-ruby-available) (candidates . ac-inf-ruby-candidates) (symbol . "r") (prefix . ac-inf-ruby-prefix)) :in-ac-modes t :auto-complete t :tab auto-complete) :offered ((:prefix "Str" :prefix-start 17 :common "Str" :menu-live t :selected "Str") (("Str" "r") ("String" "r") ("Struct" "r") ("StringIO" "r"))) :moved (:prefix "Str" :prefix-start 17 :common "Str" :menu-live t :selected "String") :completed (:text "irb(main):001:0> String" :point 23 :mode inf-ruby-mode :top-level 0 :auto-complete t :sources #1#) :unique (:text "irb(main):001:0> String\n=> nil\nirb(main):003:0> Struct" :point 54 :mode inf-ruby-mode :top-level 0 :auto-complete t :sources #1#) :requests ("Str" "Stru"))"#
    ]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

/// Half-written blocks are normal at a REPL.  While `def greet' is open the
/// process prints a continuation prompt, which is not a top-level prompt, so
/// `inf-ruby-at-top-level-prompt-p' is nil and the source's prefix function
/// declines: no menu, and above all no completion request sent into a REPL that
/// is in the middle of reading a block.  Closing the block with `end' restores
/// the top-level prompt and completion works again.
#[test]
fn ac_inf_ruby_declines_to_complete_at_a_continuation_prompt() {
    let elisp_form = r##"(ac-inf-ruby-test-with-repl
 (define-key inf-ruby-mode-map (kbd "TAB") #'auto-complete)
 (setq-local ac-sources nil)
 (ac-inf-ruby-enable)
 (auto-complete-mode 1)
 (let ((opened (ac-inf-ruby-test-submit "def greet")))
   (goto-char (point-max))
   (insert "Str")
   (let ((blocked (list :prompt opened
                        :top-level inf-ruby-at-top-level-prompt-p
                        :prefix (ac-inf-ruby-prefix)
                        :started (auto-complete)
                        :menu (ac-inf-ruby-test-menu)
                        :requests (ac-inf-ruby-test-requests))))
     (delete-region (- (point) 3) (point))
     (let ((closed (ac-inf-ruby-test-submit "end")))
       (goto-char (point-max))
       (execute-kbd-macro (kbd "S t r TAB"))
       (let ((offered (list (ac-inf-ruby-test-session) (ac-inf-ruby-test-menu))))
         (ac-abort)
         (list :blocked blocked
               :closed closed
               :top-level inf-ruby-at-top-level-prompt-p
               :offered offered
               :requests (ac-inf-ruby-test-requests)
               :after (ac-inf-ruby-test-buffer-state)))))))"##;

    let expect = expect![[
        r#"OK (:blocked (:prompt "irb(main):002:1* " :top-level nil :prefix nil :started nil :menu nil :requests nil) :closed "irb(main):003:0> " :top-level 0 :offered ((:prefix "Str" :prefix-start 74 :common "Str" :menu-live t :selected "Str") (("Str" "r") ("String" "r") ("Struct" "r") ("StringIO" "r"))) :requests ("Str") :after (:text "irb(main):001:0> def greet\nirb(main):002:1* end\n=> :done\nirb(main):003:0> Str" :point 77 :mode inf-ruby-mode :top-level 0 :auto-complete t :sources (ac-source-inf-ruby)))"#
    ]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

/// A dotted expression is where the package and the pinned inf-ruby disagree.
/// `ac-inf-ruby-prefix' hands auto-complete the whole expression, receiver
/// included, and `inf-ruby-completions' then prepends the receiver again
/// (`target') before asking the REPL -- so the process is asked to complete
/// `str.str.to_s' and answers nothing, leaving an empty menu.  The same buffer
/// position completes correctly through `inf-ruby-completions' called the way
/// inf-ruby's own completion table calls it, with the bare prefix, which shows
/// the REPL and the fixture are sound.
#[test]
fn ac_inf_ruby_asks_the_repl_for_a_doubled_receiver_on_a_dotted_expression() {
    let elisp_form = r##"(ac-inf-ruby-test-with-repl
 (define-key inf-ruby-mode-map (kbd "TAB") #'auto-complete)
 (setq-local ac-sources nil)
 (ac-inf-ruby-enable)
 (auto-complete-mode 1)
 (goto-char (point-max))
 (execute-kbd-macro (kbd "s t r . t o _ s TAB"))
 (let ((dotted (list (ac-inf-ruby-test-session)
                     (ac-inf-ruby-test-menu)
                     (inf-ruby-completion-target-at-point))))
   (ac-abort)
   (let ((package-requests (ac-inf-ruby-test-requests))
         (control (inf-ruby-completions "to_s")))
     (list :dotted dotted
           :package-requests package-requests
           :inf-ruby-completions control
           :all-requests (ac-inf-ruby-test-requests)
           :after (ac-inf-ruby-test-buffer-state)))))"##;

    let expect = expect![[
        r#"OK (:dotted ((:prefix nil :prefix-start nil :common nil :menu-live nil :selected nil) nil "str.") :package-requests ("str.str.to_s") :inf-ruby-completions ("to_s" "to_str" "to_sym") :all-requests ("str.str.to_s" "str.to_s") :after (:text "irb(main):001:0> str.to_s" :point 25 :mode inf-ruby-mode :top-level 0 :auto-complete t :sources (ac-source-inf-ruby)))"#
    ]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

/// The source's `available' predicate is `(eq 'inf-ruby-mode major-mode)', but
/// auto-complete caches the answer on the source symbol forever.  A user who
/// keeps `ac-source-inf-ruby' in a global `ac-sources' therefore poisons it the
/// first time auto-complete compiles the sources in any other buffer: the
/// property is stuck at `no', and the source stays silently disabled in the
/// REPL buffer itself, where the process is never asked anything.
#[test]
fn ac_inf_ruby_source_stays_disabled_once_it_was_compiled_outside_the_repl() {
    let elisp_form = r##"(ac-inf-ruby-test-with-repl
 (define-key inf-ruby-mode-map (kbd "TAB") #'auto-complete)
 (setq-local ac-sources nil)
 (ac-inf-ruby-enable)
 (auto-complete-mode 1)
 (let ((editor (generate-new-buffer "*project*"))
       elsewhere)
   (with-current-buffer editor
     (set-window-buffer (selected-window) editor)
     (fundamental-mode)
     (setq-local ac-sources (list 'ac-source-inf-ruby))
     (auto-complete-mode 1)
     (insert "Str")
     (setq elsewhere (list :mode major-mode
                           :started (auto-complete)
                           :menu (ac-inf-ruby-test-menu)
                           :cached (get 'ac-source-inf-ruby 'available))))
   (set-buffer "*ruby*")
   (set-window-buffer (selected-window) (current-buffer))
   (goto-char (point-max))
   (execute-kbd-macro (kbd "S t r TAB"))
   (let ((in-repl (list (ac-inf-ruby-test-session) (ac-inf-ruby-test-menu))))
     (ac-abort)
     (list :elsewhere elsewhere
           :cached (get 'ac-source-inf-ruby 'available)
           :in-repl in-repl
           :requests (ac-inf-ruby-test-requests)
           :after (ac-inf-ruby-test-buffer-state)))))"##;

    let expect = expect![[
        r#"OK (:elsewhere (:mode fundamental-mode :started nil :menu nil :cached no) :cached no :in-repl ((:prefix nil :prefix-start nil :common nil :menu-live nil :selected nil) nil) :requests nothing-recorded :after (:text "irb(main):001:0> Str" :point 20 :mode inf-ruby-mode :top-level 0 :auto-complete t :sources (ac-source-inf-ruby)))"#
    ]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

/// REPLs die.  When the inferior Ruby process is gone, the completion attempt
/// must not silently do nothing: `inf-ruby-proc' signals, and that error
/// escapes the public `auto-complete' command with the typed text still in the
/// buffer.
#[test]
fn ac_inf_ruby_reports_a_dead_repl_out_of_the_public_command() {
    let elisp_form = r##"(ac-inf-ruby-test-with-repl
 (define-key inf-ruby-mode-map (kbd "TAB") #'auto-complete)
 (setq-local ac-sources nil)
 (ac-inf-ruby-enable)
 (auto-complete-mode 1)
 (ac-inf-ruby-test-stop-repl)
 (goto-char (point-max))
 (insert "Str")
 (list :process (and (get-buffer-process (current-buffer)) t)
       :outcome (condition-case failure (auto-complete) (error failure))
       :menu (ac-inf-ruby-test-menu)
       :requests (ac-inf-ruby-test-requests)
       :after (ac-inf-ruby-test-buffer-state)))"##;

    let expect = expect![[
        r#"OK (:process nil :outcome (error "No current process. See variable inf-ruby-buffers") :menu nil :requests nothing-recorded :after (:text "irb(main):001:0> \nProcess ruby killed\nStr\n" :point 41 :mode inf-ruby-mode :top-level 0 :auto-complete t :sources (ac-source-inf-ruby)))"#
    ]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

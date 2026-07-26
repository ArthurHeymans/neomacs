use expect_test::expect;

use super::assert_abc_mode_parity;

#[test]
fn abc_set_abc2ps_option_set_uses_exact_completion_and_minibuffer_contracts() {
    let elisp_form = r##"(let ((abc-option-alist
                    '(("pretty" . "-p")
                      ("none" . "")))
                   (abc-preferred-options "old")
                   (abc-additional-options "old-extra")
                   events)
               (cl-letf
                   (((symbol-function 'completing-read)
                     (lambda (&rest arguments)
                       (push (cons 'complete arguments) events)
                       "pretty"))
                    ((symbol-function 'read-from-minibuffer)
                     (lambda (&rest arguments)
                       (push (cons 'read arguments) events)
                       "--strict")))
                 (list
                  (abc-set-abc2ps-option-set)
                  abc-preferred-options
                  abc-additional-options
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("--strict" "-p" "--strict" ((complete "Option set to use: " (("pretty" . "-p") ("none" . "")) nil t nil abc-option-history "none") (read "Additional options: " nil nil nil nil abc-additional-option-history)))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_run_abc2ps_base_preserves_side_effect_order_and_command_spacing() {
    let elisp_form = r##"(let ((abc-executable "abc ps")
                    (abc-preferred-options "-p")
                    (abc-additional-options "--extra")
                    events)
               (cl-letf
                   (((symbol-function 'save-buffer)
                     (lambda ()
                       (push '(save) events)
                       'saved))
                    ((symbol-function 'abc-preprocess-buffer)
                     (lambda (argp)
                       (push (list 'preprocess argp) events)
                       "/workspace/tunes/my tune.abc"))
                    ((symbol-function 'read-from-minibuffer)
                     (lambda (&rest arguments)
                       (push (cons 'read arguments) events)
                       "edited command"))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)
                       'shell-result)))
                 (list
                  (abc-run-abc2ps-base 'prefix nil)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (shell-result ((save) (preprocess prefix) (read "Options: " "abc ps -p --extra my tune.abc -O =") (shell "edited command")))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_run_abc2ps_base_inserts_options_without_normalizing_whitespace() {
    let elisp_form = r##"(let ((abc-executable "abcm2ps")
                    (abc-preferred-options "")
                    (abc-additional-options "")
                    defaults)
               (cl-letf
                   (((symbol-function 'save-buffer) #'ignore)
                    ((symbol-function 'abc-preprocess-buffer)
                     (lambda (_) "/a/song.abc"))
                    ((symbol-function 'read-from-minibuffer)
                     (lambda (_prompt default &rest _)
                       (push default defaults)
                       default))
                    ((symbol-function 'shell-command) #'identity))
                 (list
                  (abc-run-abc2ps-base nil nil)
                  (abc-run-abc2ps-base nil "")
                  (abc-run-abc2ps-base nil "--landscape")
                  (nreverse defaults))))"##;
    let expect = expect![[
        r#"OK ("abcm2ps   song.abc -O =" "abcm2ps   song.abc -O =" "abcm2ps--landscape   song.abc -O =" ("abcm2ps   song.abc -O =" "abcm2ps   song.abc -O =" "abcm2ps--landscape   song.abc -O ="))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_run_abc2ps_wrappers_forward_exact_all_and_current_song_options() {
    let elisp_form = r##"(with-temp-buffer
               (insert "X:17\nT:Song\n")
               (goto-char (point-max))
               (let (events)
                 (cl-letf
                     (((symbol-function 'abc-run-abc2ps-base)
                       (lambda (&rest arguments)
                         (push arguments events)
                         (length events))))
                   (list
                    (abc-run-abc2ps-all 'all-prefix nil)
                    (abc-run-abc2ps-all nil "--all")
                    (abc-run-abc2ps-one 'one-prefix nil)
                    (abc-run-abc2ps-one nil "--one")
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (1 2 3 4 ((all-prefix "") (nil "--all") (one-prefix " -e 17") (nil " -e 17--one")))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_set_preprocess_options_returns_and_stores_the_exact_string() {
    let elisp_form = r##"(let ((abc-pp-options "before"))
               (list
                (abc-set-preprocess-options "--define=α --flag")
                abc-pp-options))"##;
    let expect = expect![[r#"OK ("--define=α --flag" "--define=α --flag")"#]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_preprocess_is_a_noop_without_an_executable_or_for_non_abp_names() {
    let elisp_form = r##"(let ((abc-pp-options "old")
                    events)
               (cl-letf
                   (((symbol-function 'read-from-minibuffer)
                     (lambda (&rest arguments)
                       (push (cons 'read arguments) events)
                       "new"))
                    ((symbol-function 'call-process)
                     (lambda (&rest arguments)
                       (push (cons 'call arguments) events)))
                    ((symbol-function 'write-region)
                     (lambda (&rest arguments)
                       (push (cons 'write arguments) events))))
                 (list
                  (let ((abc-pp-executable nil))
                    (abc-preprocess t "/music/source.abp"))
                  abc-pp-options
                  (let ((abc-pp-executable "abcpp"))
                    (abc-preprocess nil "/music/source.abc" t))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("/music/source.abp" "new" "/music/source.abc" ((read "Preprocesor Options: " "old")))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_preprocess_abp_dispatches_empty_options_and_writes_exact_buffer_range() {
    let elisp_form = r##"(let ((abc-pp-executable "abcpp")
                    (abc-pp-options "")
                    events)
               (cl-letf
                   (((symbol-function 'call-process)
                     (lambda (&rest arguments)
                       (push (cons 'call arguments) events)
                       (insert "generated α")
                       23))
                    ((symbol-function 'write-region)
                     (lambda (start end name &rest arguments)
                       (push
                        (list
                         'write start end name arguments
                         (buffer-string))
                        events)
                       'written)))
                 (list
                  (abc-preprocess nil "/music/source.abp")
                  abc-pp-options
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("/music/source.abp.abc" "" ((call "abcpp" nil t t "/music/source.abp") (write 1 11 "/music/source.abp.abc" nil "generated α")))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_preprocess_prompted_options_and_midi_macro_have_distinct_call_shapes() {
    let elisp_form = r##"(let ((abc-pp-executable "abcpp")
                    (abc-pp-options "--old")
                    (abc-pp-midi-macro "-MIDI")
                    events)
               (cl-letf
                   (((symbol-function 'read-from-minibuffer)
                     (lambda (&rest arguments)
                       (push (cons 'read arguments) events)
                       "--new"))
                    ((symbol-function 'call-process)
                     (lambda (&rest arguments)
                       (push (cons 'call arguments) events)
                       (insert "output")
                       0))
                    ((symbol-function 'write-region)
                     (lambda (start end name &rest arguments)
                       (push
                        (list 'write start end name arguments)
                        events)
                       nil)))
                 (list
                  (abc-preprocess t "/music/a.abp")
                  abc-pp-options
                  (abc-preprocess nil "/music/b.abp" t)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("/music/a.abp.abc" "--new" "/music/b.abp.abc" ((read "Preprocesor Options: " "--old") (call "abcpp" nil t t "--new" "/music/a.abp") (write 1 6 "/music/a.abp.abc" nil) (call "abcpp" nil t t "--new" "-MIDI" "/music/b.abp") (write 1 6 "/music/b.abp.abc" nil)))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_preprocess_buffer_forwards_prefix_and_current_file_name() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/workspace/current.abp")
               (cl-letf
                   (((symbol-function 'abc-preprocess)
                     (lambda (&rest arguments)
                       arguments)))
                 (abc-preprocess-buffer '(4))))"##;
    let expect = expect![[r#"OK ((4) "/workspace/current.abp")"#]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_run_midi_base_and_song_wrapper_preserve_exact_spacing_and_order() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/workspace/tune.abp")
               (insert "X:27\nT:Song\n")
               (goto-char (point-max))
               (let ((abc-midi-executable "abc2midi")
                     events)
                 (cl-letf
                     (((symbol-function 'save-buffer)
                       (lambda ()
                         (push '(save) events)))
                      ((symbol-function 'abc-preprocess)
                       (lambda (&rest arguments)
                         (push (cons 'preprocess arguments) events)
                         "/built/tune.abp.abc"))
                      ((symbol-function 'read-from-minibuffer)
                       (lambda (&rest arguments)
                         (push (cons 'read arguments) events)
                         (cadr arguments)))
                      ((symbol-function 'shell-command)
                       (lambda (command)
                         (push (list 'shell command) events)
                         'shell-result)))
                   (list
                    (abc-run-abc2midi 'base-prefix nil)
                    (abc-run-abc2midi nil "--tempo 90")
                    (abc-run-abc2midi-one 'song-prefix nil)
                    (abc-run-abc2midi-one nil "--bars")
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (shell-result shell-result shell-result shell-result (#1=(save) (preprocess base-prefix "/workspace/tune.abp" t) (read "Command: " "abc2midi tune.abp.abc") (shell "abc2midi tune.abp.abc") #1# (preprocess nil "/workspace/tune.abp" t) (read "Command: " "abc2midi tune.abp.abc --tempo 90") (shell "abc2midi tune.abp.abc --tempo 90") #1# (preprocess song-prefix "/workspace/tune.abp" t) (read "Command: " "abc2midi tune.abp.abc  27") (shell "abc2midi tune.abp.abc  27") #1# (preprocess nil "/workspace/tune.abp" t) (read "Command: " "abc2midi tune.abp.abc  27 --bars") (shell "abc2midi tune.abp.abc  27 --bars")))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

#[test]
fn abc_run_abc2abc_uses_preprocessed_basename_and_exact_prompt() {
    let elisp_form = r##"(let ((abc-abc2abc-executable "abc2abc")
                    events)
               (cl-letf
                   (((symbol-function 'save-buffer)
                     (lambda ()
                       (push '(save) events)))
                    ((symbol-function 'abc-preprocess-buffer)
                     (lambda (argp)
                       (push (list 'preprocess argp) events)
                       "/generated/song.abc"))
                    ((symbol-function 'read-from-minibuffer)
                     (lambda (&rest arguments)
                       (push (cons 'read arguments) events)
                       "chosen"))
                    ((symbol-function 'shell-command)
                     (lambda (command)
                       (push (list 'shell command) events)
                       'shell-result)))
                 (list
                  (abc-run-abc2abc '(4))
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (shell-result ((save) (preprocess (4)) (read "Command: " "abc2abc song.abc") (shell "chosen")))"#
    ]];

    assert_abc_mode_parity(elisp_form, expect);
}

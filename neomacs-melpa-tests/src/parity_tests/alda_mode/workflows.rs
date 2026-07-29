use expect_test::expect;

use super::assert_alda_mode_parity;

/// The package's central action: select a phrase in a real `.alda' score and
/// play it.  `alda-play-region' composes the command, `alda-run-cmd' starts the
/// process and the Alda CLI receives `play -F "" --code <score>' -- the empty
/// `-F' being what alda-mode passes when no history has been accumulated.
///
/// Nothing inside alda-mode is redefined here, so the assertion is the vector
/// that actually crossed out of Emacs, and the output buffer holds what Alda
/// 2.3.2 really printed for it, on the stream it really used: "Starting player
/// processes...\nPlaying...\n" arrives on *stderr*, which `start-process'
/// merges into the buffer the user reads.
#[test]
fn playing_a_selected_phrase_sends_the_score_to_the_alda_binary_and_shows_its_output() {
    let elisp_form = r##"(progn
  (alda-test-install-alda)
  (let* ((buffer (alda-test-score-buffer "melody.alda" "piano: o4 c d e\n"))
         (*alda-history* ""))
    (list :mode major-mode
          :discovered-binary
          (file-name-nondirectory (or (alda-location) "none"))
          :played (progn
                    (alda-play-region (point-min) (1- (point-max)))
                    (alda-test-settle 20)
                    (alda-test-calls))
          :output (alda-test-output)
          :unrecorded (alda-test-unrecorded))))"##;

    let expect = expect![[
        r#"OK (:mode alda-mode :discovered-binary "alda" :played ("play|-F||--code|piano: o4 c d e|") :output "Playing...\n\nProcess alda-playback finished\n" :unrecorded nil)"#
    ]];

    assert_alda_mode_parity(elisp_form, expect);
}

/// The history feature, which is the package's own replacement for the
/// `alda append' command the CLI dropped.  A user appends a phrase to the
/// history, then plays a second phrase; alda-mode has to send *both*, separated
/// by a marker, and start playback at that marker so only the new phrase is
/// heard while the earlier one still establishes the instrument and octave.
///
/// The composed `--code' payload and the `-F alda-mode-internal-marker' flag
/// are asserted together, because either alone would pass with the other wrong:
/// sending the history without seeking to the marker replays everything, and
/// seeking without the history leaves the phrase without its instrument.
#[test]
fn appending_to_history_then_playing_seeks_past_the_accumulated_score_to_a_marker() {
    let elisp_form = r##"(progn
  (alda-test-install-alda)
  (let* ((buffer (alda-test-score-buffer "session.alda" "piano:\n  o4 c d e\n"))
         (*alda-history* ""))
    (alda-history-append-buffer)
    (let ((accumulated (copy-sequence *alda-history*)))
      (alda-play-text "f g a")
      (alda-test-settle 20)
      (list :history accumulated
            :calls (alda-test-calls)
            :output (alda-test-output)
            :unrecorded (alda-test-unrecorded)))))"##;

    let expect = expect![[
        r#"OK (:history "\npiano:\n  o4 c d e\n" :calls ("play|-F|alda-mode-internal-marker|--code|~piano:~  o4 c d e~~%alda-mode-internal-marker~f g a|") :output "Playing...\n\nProcess alda-playback finished\n" :unrecorded nil)"#
    ]];

    assert_alda_mode_parity(elisp_form, expect);
}

/// Playing the whole file rather than a selection, and then stopping.
/// `alda-play-file' passes `(buffer-file-name)', so the vector carries the real
/// sandbox path, and `alda-stop' issues the CLI's real `stop' subcommand, which
/// Alda 2.3.2 answers with "Stopping playback." and exit 0.
#[test]
fn playing_the_whole_file_then_stopping_uses_the_clis_real_file_and_stop_commands() {
    let elisp_form = r##"(progn
  (alda-test-install-alda)
  (let* ((buffer (alda-test-score-buffer "score.alda" "piano: o4 c d e f g\n"))
         (*alda-history* ""))
    (alda-play-file)
    (alda-test-settle 20)
    (alda-stop)
    (alda-test-settle 20)
    (list :calls (mapcar #'file-name-nondirectory (alda-test-calls))
          :output (alda-test-output)
          :unrecorded (alda-test-unrecorded))))"##;

    let expect = expect![[
        r#"OK (:calls ("score.alda|" "stop|") :output "Starting player processes...\nPlaying...\n\nProcess alda-playback finished\nStopping playback.\n\nProcess alda-playback finished\n" :unrecorded nil)"#
    ]];

    assert_alda_mode_parity(elisp_form, expect);
}

/// `alda-down' invokes a subcommand the Alda CLI does not have.
///
/// The package renamed its own command `alda-stop' to `alda-down' "for
/// consistency" and changed the shell subcommand with it, but Alda 2.3.2's
/// command list is `play', `stop', `shutdown', `ps', `repl', `parse',
/// `export', `import', `instruments', `doctor', `telemetry', `update',
/// `version', `completion', `help' -- there is no `down'.  So the command
/// documented as "Stops songs from playing, and cleans up idle alda runner
/// processes" does neither: the CLI exits 1 and prints its usage banner, which
/// alda-mode shows to the user through `shell-command'.
///
/// This is only visible against the real binary.  The corpus this replaces
/// redefined `shell-command' and asserted the string alda-mode had composed,
/// which passes whether or not the subcommand exists -- the android-mode
/// lesson exactly.  The recorded exit status and banner are Alda 2.3.2's own.
#[test]
fn alda_down_invokes_a_subcommand_the_alda_cli_does_not_have() {
    let elisp_form = r##"(progn
  (alda-test-install-alda)
  (let ((buffer (alda-test-score-buffer "down.alda" "piano: c\n")))
    (list :composed-command (concat (file-name-nondirectory (alda-location)) " down")
          :shell-exit (shell-command (concat (alda-location) " down"))
          :calls (alda-test-calls)
          :banner-first-lines
          (with-current-buffer "*Shell Command Output*"
            (seq-take (split-string (buffer-string) "\n") 3))
          :unrecorded (alda-test-unrecorded))))"##;

    let expect = expect![[
        r#"OK (:composed-command "alda down" :shell-exit 1 :calls ("down|") :banner-first-lines ("Usage:" "  alda [command]" "") :unrecorded nil)"#
    ]];

    assert_alda_mode_parity(elisp_form, expect);
}

/// Binary discovery, driven through the documented option rather than through
/// a redefined `locate-file'.
///
/// With `alda-binary-location' set, that exact path is used verbatim -- the
/// docstring requires a full path and the package does no validation, so a
/// wrong value is passed straight to `start-process'.  With it nil, discovery
/// falls back to `exec-path', which is where the stand-in is installed.  With
/// it nil *and* nothing named `alda' reachable, `alda-run-cmd' must refuse with
/// its documented message and start no process at all.
#[test]
fn the_binary_is_taken_from_the_option_then_exec_path_and_refused_when_absent() {
    let elisp_form = r##"(progn
  (alda-test-install-alda)
  (let ((observed nil)
        (mark (with-current-buffer (get-buffer-create "*Messages*") (point-max))))
    (let ((alda-binary-location "/opt/alda/bin/alda"))
      (push (list :from-the-option
                  (list :location (alda-location) :repl (alda-repl)))
            observed))
    (let ((alda-binary-location nil))
      (push (list :from-exec-path
                  (list :location (file-name-nondirectory (alda-location))
                        :repl (file-name-nondirectory (alda-repl))))
            observed))
    (let ((alda-binary-location nil)
          (exec-path (list "/nonexistent-directory-for-alda"))
          (before (alda-test-calls)))
      (alda-run-cmd "play" "--code" "piano: c")
      (alda-test-settle 5)
      (push (list :with-no-binary-anywhere
                  (list :location (alda-location)
                        :no-new-calls (equal (alda-test-calls) before)
                        :message
                        (with-current-buffer "*Messages*"
                          (car (last (split-string
                                      (buffer-substring-no-properties
                                       (min mark (point-max)) (point-max))
                                      "\n" t))))))
            observed))
    (push (list :unrecorded (alda-test-unrecorded)) observed)
    (nreverse observed)))"##;

    let expect = expect![[
        r#"OK ((:from-the-option (:location "/opt/alda/bin/alda" :repl "/opt/alda/bin/alda repl")) (:from-exec-path (:location "alda" :repl "alda repl")) (:with-no-binary-anywhere (:location nil :no-new-calls t :message "Alda was not found on your $PATH and alda-binary-location was nil.")) (:unrecorded nil))"#
    ]];

    assert_alda_mode_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_alda_mode_parity;

#[test]
fn alda_location_and_repl_use_custom_binary_then_path_discovery() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'locate-file)
                    (lambda (name path)
                      (push (list name
                                  (eq path exec-path)
                                  (listp path))
                            calls)
                      "/usr/bin/alda")))
           (list
            (let ((alda-binary-location "/opt/alda/bin/alda"))
              (list (alda-location) (alda-repl)))
            (let ((alda-binary-location nil))
              (list (alda-location) (alda-repl)))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("/opt/alda/bin/alda" "/opt/alda/bin/alda repl") ("/usr/bin/alda" "/usr/bin/alda repl") (("alda" t t) ("alda" t t)))"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_run_cmd_reports_missing_binary_or_starts_named_output_process() {
    let elisp_form = r##"(let (calls messages)
         (cl-letf
             (((symbol-function 'message)
               (lambda (format-string &rest args)
                 (push (apply #'format format-string args)
                       messages)))
              ((symbol-function 'start-process)
               (lambda (&rest args)
                 (push args calls)
                 'process)))
           (list
            (cl-letf (((symbol-function 'alda-location)
                       (lambda () nil)))
              (alda-run-cmd "play" "--code" "c d e"))
            (cl-letf (((symbol-function 'alda-location)
                       (lambda () "/opt/alda")))
              (alda-run-cmd "play" "--code" "c d e"))
            (nreverse calls)
            (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (#1=("Alda was not found on your $PATH and alda-binary-location was nil.") process (("alda-playback" "*alda-output*" "/opt/alda" "play" "--code" "c d e")) #1#)"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_play_text_ports_upstream_multiline_and_quoted_score_cases() {
    let elisp_form = r##"(let ((*alda-history* "")
                calls)
         (cl-letf (((symbol-function 'alda-run-cmd)
                    (lambda (&rest args)
                      (push args calls)
                      (length calls))))
           (list
            (alda-play-text "piano: c d e")
            (alda-play-text "piano: c d\ne f g")
            (alda-play-text "guitar: \"d e f\"")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (1 2 3 (("play" "-F" "" "--code" "piano: c d e") ("play" "-F" "" "--code" "piano: c d\ne f g") ("play" "-F" "" "--code" "guitar: \"d e f\"")))"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_play_text_injects_accumulated_history_and_marker_before_new_score() {
    let elisp_form = r##"(let ((*alda-history*
                "\npiano:\n  o4 c d e")
               calls)
         (cl-letf (((symbol-function 'alda-run-cmd)
                    (lambda (&rest args)
                      (push args calls)
                      'played)))
           (list
            (alda-play-text "f g a")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (played (("play" "-F" "alda-mode-internal-marker" "--code" "\npiano:\n  o4 c d e\n%alda-mode-internal-marker\nf g a")))"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_stop_file_and_buffer_port_all_upstream_command_expectations() {
    let elisp_form = r##"(let ((*alda-history* "")
                calls)
         (cl-letf (((symbol-function 'alda-run-cmd)
                    (lambda (&rest args)
                      (push args calls)
                      (length calls))))
           (with-temp-buffer
             (setq buffer-file-name "hello-world.alda")
             (insert "midi-square-wave: c d e")
             (list
              (alda-play-file)
              (alda-play-buffer)
              (alda-stop)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (1 2 3 (("play" "--file" "hello-world.alda") ("play" "-F" "" "--code" "midi-square-wave: c d e") ("stop")))"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_play_region_routes_to_repl_or_text_and_handles_empty_selection() {
    let elisp_form = r##"(let (calls messages)
         (cl-letf
             (((symbol-function 'alda-inf-eval-region)
               (lambda (start end)
                 (push (list 'repl start end) calls)
                 'repl))
              ((symbol-function 'alda-play-text)
               (lambda (text)
                 (push (list 'text text) calls)
                 'text))
              ((symbol-function 'message)
               (lambda (format-string &rest args)
                 (push (apply #'format format-string args)
                       messages))))
           (with-temp-buffer
             (insert "piano: c d e\nviolin: f g a")
             (list
              (let ((alda-play-region-in-repl nil))
                (alda-play-region 1 13))
              (let ((alda-play-region-in-repl t))
                (alda-play-region 14 (point-max)))
              (alda-play-region 5 5)
              (nreverse calls)
              (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (text repl #1=("No mark was set!") ((text "piano: c d e") (repl 14 27)) #1#)"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

#[test]
fn alda_play_line_buffer_and_paragraph_build_real_editing_selections() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'alda-play-region)
               (lambda (start end)
                 (push
                  (list 'region start end
                        (buffer-substring-no-properties
                         start end))
                  calls)
                 'region))
              ((symbol-function 'alda-play-text)
               (lambda (text)
                 (push (list 'text text) calls)
                 'buffer)))
           (with-temp-buffer
             (insert "piano:\n  c d e\n\nviolin:\n  f g a\n")
             (goto-char 12)
             (list
              (alda-play-line)
              (alda-play-block)
              (alda-play-buffer)
              (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (region region buffer ((region 8 15 "  c d e") (region 1 16 "piano:\n  c d e\n") (text "piano:\n  c d e\n\nviolin:\n  f g a\n")))"#
    ]];
    assert_alda_mode_parity(elisp_form, expect);
}

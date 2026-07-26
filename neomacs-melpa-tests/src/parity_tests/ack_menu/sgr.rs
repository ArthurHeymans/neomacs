use super::assert_ack_menu_parity;
use expect_test::expect;

#[test]
fn ack_menu_parse_sgr_fragment_covers_offsets_boundaries_and_multiple_escapes() {
    let elisp_form = r##"(list
         (ack-parse-sgr-fragment
          "plain")
         (ack-parse-sgr-fragment
          "abc\033123456789")
         (ack-parse-sgr-fragment
          "abc\0331234567890")
         (ack-parse-sgr-fragment
          "x\033short"
          2)
         (ack-parse-sgr-fragment
          "x\033one\033two"))"##;
    let expect = expect![[
        r#"OK (("plain") ("abc" . "\033123456789") ("abc\0331234567890") ("x\33short") ("x" . "\33one\33two"))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_parse_sgr_sequences_decodes_known_regions_and_drops_unknown_escapes() {
    let elisp_form = r##"(progn
         (defvar ansi-color-regexp)
         (defvar ansi-color-drop-regexp)
         (let ((ansi-color-regexp
                "\033\\[\\([0-9;]*m\\)")
               (ansi-color-drop-regexp
                "\033\\[\\([ABCDsuK]\\|[12][JK]\\|=[0-9]+[hI]\\|[0-9;]*[Hf]\\|\\?[0-9]+[hl]\\)")
               (ack-parse-sgr-context
                nil)
               calls)
         (cl-labels
             ((capture
               (text code)
               (push
                (list text code)
                calls)
               (concat
                "<"
                text
                ">")))
           (list
            (ack-parse-sgr-sequences
             "pre\033[2J\033[1;33m12\033[0m-mid-\033[1;32mfile\033[m-post"
             #'capture)
            (nreverse calls)
            (copy-tree
             ack-parse-sgr-context)))))"##;
    let expect =
        expect![[r#"OK ("pre<12>-mid-<file>-post" (("12" "1;33m") ("file" "1;32m")) (nil))"#]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_parse_sgr_sequences_streams_incomplete_fragments_across_calls() {
    let elisp_form = r##"(progn
         (defvar ansi-color-regexp)
         (defvar ansi-color-drop-regexp)
         (let ((ansi-color-regexp
                "\033\\[\\([0-9;]*m\\)")
               (ansi-color-drop-regexp
                "\033\\[\\([ABCDsuK]\\|[12][JK]\\|=[0-9]+[hI]\\|[0-9;]*[Hf]\\|\\?[0-9]+[hl]\\)")
               (ack-parse-sgr-context
                nil)
               calls)
         (cl-labels
             ((capture
               (text code)
               (push
                (list text code)
                calls)
               (upcase text)))
           (let ((first
                  (ack-parse-sgr-sequences
                   "head\033[30;43"
                   #'capture))
                 second
                 third)
             (setq second
                   (ack-parse-sgr-sequences
                    "mmatch"
                    #'capture)
                   third
                   (ack-parse-sgr-sequences
                    "\033[0mtail"
                    #'capture))
             (list
              first
              second
              third
              (nreverse calls)
              (copy-tree
               ack-parse-sgr-context))))))"##;
    let expect = expect![[r#"OK ("head" "" "MATCHtail" (("match" "30;43m")) (nil))"#]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_parse_sgr_finish_flushes_colored_plain_and_empty_contexts() {
    let elisp_form = r##"(let (calls)
         (cl-labels
             ((capture
               (text code)
               (push
                (list text code)
                calls)
               (concat
                code
                ":"
                text)))
           (list
            (let ((ack-parse-sgr-context
                   '("30;43m"
                     .
                     "tail")))
              (list
               (ack-parse-sgr-sequences-finish
                #'capture)
               ack-parse-sgr-context))
            (let ((ack-parse-sgr-context
                   '(nil
                     .
                     "plain")))
              (ack-parse-sgr-sequences-finish
               #'capture))
            (let ((ack-parse-sgr-context
                   nil))
              (ack-parse-sgr-sequences-finish
               #'capture))
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (("30;43m:tail" nil) "plain" "" (("tail" "30;43m")))"#]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_apply_faces_sets_exact_properties_for_every_supported_sgr_code() {
    let elisp_form = r##"(mapcar
         (lambda (code)
           (let* ((input
                   (copy-sequence
                    "value"))
                  (result
                   (ack-apply-faces
                    input
                    code)))
             (list
              code
              (eq input result)
              (substring-no-properties
               result)
              (text-properties-at
               0 result))))
         '("1;33m"
           "1;32m"
           "30;43m"
           "31m"))"##;
    let expect = expect![[
        r#"OK (("1;33m" t "value" (ack-line #("value" 0 5 (ack-line #1=#("value" 0 5 (ack-line #1# font-lock-face ack-line)) font-lock-face ack-line)) font-lock-face ack-line)) ("1;32m" t "value" (ack-file #("value" 0 5 (ack-file #2=#("value" 0 5 (ack-file #2# font-lock-face ack-file)) font-lock-face ack-file)) font-lock-face ack-file)) ("30;43m" t "value" (follow-link t mouse-face highlight ack-match t font-lock-face ack-match)) ("31m" t "value" nil))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_count_matches_counts_property_runs_not_characters() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "0123456789")
         (add-text-properties
          1 4
          '(ack-match t))
         (add-text-properties
          6 7
          '(ack-match first))
         (add-text-properties
          7 9
          '(ack-match second))
         (list
          (ack-count-matches)
          (mapcar
           (lambda (position)
             (get-text-property
              position
              'ack-match))
           (number-sequence
            1 10))))"##;
    let expect = expect!["OK (2 (t t t nil nil first second second nil nil))"];
    assert_ack_menu_parity(elisp_form, expect);
}

use expect_test::expect;

use super::{assert_ac_racer_parity, assert_ac_racer_signal_parity};

#[test]
fn ac_racer_collect_candidates_parses_every_match_field_and_preserves_popup_properties() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "noise\nMATCH alpha,1,2,/project/a.rs,Struct,struct alpha { value: i32 }\nMATCH βeta,3,4,/project/b.rs,Function,fn βeta(one: i32, two: i32) -> i32\nMATCH alpha,5,6,/project/c.rs,Module,mod alpha\ntrailer\n")
               (goto-char
                12)
               (set-match-data
                '(2 4))
               (let ((items
                      (ac-racer--collect-candidates)))
                 (list
                  (point)
                  (list
                   (match-beginning
                    0)
                   (match-end
                    0)
                   (match-beginning
                    1)
                   (match-end
                    1)
                   (match-beginning
                    2)
                   (match-end
                    2)
                   (match-beginning
                    3)
                   (match-end
                    3))
                  items
                  (mapcar
                   (lambda (item)
                     (list
                      (substring-no-properties
                       item)
                      (text-properties-at
                       0
                       item)
                      (popup-item-document
                       item)
                      (popup-item-summary
                       item)))
                   items))))"##;
    let expect = expect![[
        r#"OK (191 (145 191 151 156 175 181 182 191) (#("alpha" 0 5 (document "struct alpha { value: i32 }" summary "Struct")) #("βeta" 0 4 (document "fn βeta(one: i32, two: i32) -> i32" summary "Function")) #("alpha" 0 5 (document "mod alpha" summary "Module"))) (("alpha" (document "struct alpha { value: i32 }" summary "Struct") "struct alpha { value: i32 }" "Struct") ("βeta" (document "fn βeta(one: i32, two: i32) -> i32" summary "Function") "fn βeta(one: i32, two: i32) -> i32" "Function") ("alpha" (document "mod alpha" summary "Module") "mod alpha" "Module")))"#
    ]];

    assert_ac_racer_parity(elisp_form, expect);
}

#[test]
fn ac_racer_collect_candidates_rejects_partial_lines_and_exposes_comma_delimited_path_limit() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                " MATCH leading,1,2,/x,Type,signature\nMATCH missing-signature,1,2,/x,Type,\nMATCH missing-type,1,2,/x,,signature\nMATCH comma-path,1,2,/project,with,comma.rs,Function,fn comma_path()\nMATCH exact,1,2,/x,Type,signature\n")
               (let ((items
                      (ac-racer--collect-candidates)))
                 (mapcar
                  (lambda (item)
                    (list
                     (substring-no-properties
                      item)
                     (popup-item-summary
                      item)
                     (popup-item-document
                      item)))
                  items)))"##;
    let expect = expect![[
        r#"OK (("comma-path" "with" "comma.rs,Function,fn comma_path()") ("exact" "Type" "signature"))"#
    ]];

    assert_ac_racer_parity(elisp_form, expect);
}

#[test]
fn ac_racer_prefix_covers_word_symbol_punctuation_unicode_middle_narrowing_and_match_data() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (fundamental-mode)
                   (insert
                    (car fixture))
                   (goto-char
                    (cadr fixture))
                   (when
                       (caddr fixture)
                     (narrow-to-region
                      (caddr fixture)
                      (cadddr fixture)))
                   (set-match-data
                    '(1 2))
                   (let ((before
                          (point))
                         (prefix
                          (ac-racer--prefix)))
                     (list
                      (car fixture)
                      before
                      prefix
                      (point)
                      (match-data)))))
               '(("alpha_beta9"
                  12)
                 ("left-right"
                  11)
                 ("two words"
                  10)
                 ("λambda_δ"
                  9)
                 ("alpha_beta"
                  7)
                 ("xxalpha_beta"
                  13
                  3
                  13)
                 (" punctuation"
                  2)))"##;
    let expect = expect![[
        r#"OK (("alpha_beta9" 12 1 12 (1 2)) ("left-right" 11 1 11 (1 2)) ("two words" 10 5 10 (1 2)) ("λambda_δ" 9 1 9 (1 2)) ("alpha_beta" 7 1 7 (1 2)) ("xxalpha_beta" 13 3 13 (1 2)) (" punctuation" 2 2 2 (1 2)))"#
    ]];

    assert_ac_racer_parity(elisp_form, expect);
}

#[test]
fn ac_racer_candidates_write_snapshot_inject_rust_source_and_parse_success_output() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "fn main() {\n    pri\n}\n")
               (goto-char
                (point-max))
               (let ((buffer-file-name
                      "/project/main.rs")
                     (racer-rust-src-path
                      "/rust/source")
                     (racer-cmd
                      "/tools/racer")
                     (ac-racer--tempfile
                      "/cache/ac-racer-input")
                     calls)
                 (cl-letf
                     (((symbol-function
                        'write-region)
                       (lambda
                           (start end file append visit)
                         (push
                          (list
                           'write
                           start
                           end
                           file
                           append
                           visit
                           (buffer-substring-no-properties
                            start end))
                          calls)
                         'written))
                      ((symbol-function
                        'process-file)
                       (lambda
                           (program infile destination display
                            &rest arguments)
                         (push
                          (list
                           'process
                           program
                           infile
                           destination
                           display
                           arguments
                           (getenv
                            "RUST_SRC_PATH"))
                          calls)
                         (insert
                          "MATCH print,2,7,/rust/print.rs,Function,fn print(value: &str)\n")
                         0)))
                   (let ((result
                          (ac-racer--candidates)))
                     (list
                      result
                      (mapcar
                       (lambda (item)
                         (list
                          (substring-no-properties
                           item)
                          (popup-item-summary
                           item)
                          (popup-item-document
                           item)))
                       result)
                      (nreverse calls)
                      (getenv
                       "RUST_SRC_PATH"))))))"##;
    let expect = expect![[
        r#"OK ((#("print" 0 5 (document "fn print(value: &str)" summary "Function"))) (("print" "Function" "fn print(value: &str)")) ((write 1 23 "/cache/ac-racer-input" nil no-message "fn main() {\n    pri\n}\n") (process "/tools/racer" nil t nil ("complete" "4" "0" "/project/main.rs" "/cache/ac-racer-input") "/rust/source")) nil)"#
    ]];

    assert_ac_racer_parity(elisp_form, expect);
}

#[test]
fn ac_racer_candidates_nonzero_status_skips_collection_and_preserves_existing_environment() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "mod demo;")
               (let ((process-environment
                      (copy-sequence
                       process-environment))
                     (racer-rust-src-path
                      nil)
                     (racer-cmd
                      "racer")
                     (ac-racer--tempfile
                      "/cache/input")
                     (status
                      7)
                     calls)
                 (setenv
                  "RUST_SRC_PATH"
                  "/existing/source")
                 (cl-letf
                     (((symbol-function
                        'write-region)
                       (lambda (&rest arguments)
                         (push
                          (cons
                           'write
                           arguments)
                          calls)
                         'written))
                      ((symbol-function
                        'process-file)
                       (lambda (&rest arguments)
                         (push
                          (list
                           'process
                           arguments
                           (getenv
                            "RUST_SRC_PATH"))
                          calls)
                         status))
                      ((symbol-function
                        'ac-racer--collect-candidates)
                       (lambda ()
                         (push
                          '(collect)
                          calls)
                         'collected)))
                   (let ((existing
                          (ac-racer--candidates)))
                     (setq
                      racer-rust-src-path
                      ""
                      status
                      9)
                     (let ((empty-override
                            (ac-racer--candidates)))
                       (list
                        existing
                        empty-override
                        (getenv
                         "RUST_SRC_PATH")
                        (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (nil nil "/existing/source" ((write 1 10 "/cache/input" nil no-message) (process ("racer" nil t nil "complete" "1" "9" "" "/cache/input") "/existing/source") (write 1 10 "/cache/input" nil no-message) (process ("racer" nil t nil "complete" "1" "9" "" "/cache/input") "")))"#
    ]];

    assert_ac_racer_parity(elisp_form, expect);
}

#[test]
fn ac_racer_candidates_exposes_non_numeric_process_status_signal() {
    let elisp_form = r##"(let ((racer-rust-src-path
                    nil))
               (cl-letf
                   (((symbol-function
                      'write-region)
                     (lambda (&rest _arguments)
                       nil))
                    ((symbol-function
                      'process-file)
                     (lambda (&rest _arguments)
                       "0")))
                 (ac-racer--candidates)))"##;
    let expect = expect![[r#"ERR (wrong-type-argument number-or-marker-p "0")"#]];

    assert_ac_racer_signal_parity(elisp_form, expect);
}

use super::assert_assess_parity;
use expect_test::{Expect, expect};

#[test]
fn assess_equality_compares_content_across_strings_and_buffers_and_ignores_properties() {
    let elisp_form = r##"
(assess-with-temp-buffers
    ((left
      (insert
       (propertize
        "alpha\nbeta"
        'fixture-face
        'bold)))
     (same
      (insert "alpha\nbeta"))
     (different
      (insert "alpha\nBETA")))
  (list
   (assess= "alpha\nbeta" left)
   (assess= left same)
   (assess= left different)
   (assess=
    (propertize
     "alpha\nbeta"
     'fixture
     1)
    "alpha\nbeta")
   (get 'assess= 'ert-explainer)))
"##;
    let expect: Expect = expect!["OK (t t nil t assess-explain=)"];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn simple_explainer_preserves_both_values_in_its_diagnostic() {
    let elisp_form = r##"
(list
 (assess--explainer-simple-string=
  "line one\nline two"
  "line one\nLINE TWO")
 (assess--explainer-simple-string=
  ""
  "λ"))
"##;
    let expect: Expect = expect![[
        r#"OK ("String :line one\nline two:line one\nLINE TWO: are not equal." "String ::λ: are not equal.")"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn diff_explainer_writes_both_inputs_and_embeds_process_output() {
    let elisp_form = r##"
(let (calls)
  (cl-letf
      (((symbol-function 'executable-find)
        (lambda (program)
          (push (list :find program) calls)
          "/fixture/bin/diff"))
       ((symbol-function 'call-process)
        (lambda (program infile destination display &rest args)
          (push
           (list
            :call
            program
            infile
            destination
            display
            (car args)
            (assess-test-read-file (cadr args))
            (assess-test-read-file (caddr args)))
           calls)
          (insert
           "*** fixture-a\n--- fixture-b\n! changed\n")
          1)))
    (let ((diagnostic
           (assess--explainer-diff-string=
            "alpha\nold\n"
            "alpha\nnew\n")))
      (list
       diagnostic
       (nreverse calls)
       (seq-every-p
        (lambda (buffer)
          (not
           (member
            (buffer-name buffer)
            '("a" "b"))))
        (buffer-list))))))
"##;
    let expect: Expect = expect![[
        r#"OK ("Strings:\nalpha\nold\n\nand\nalpha\nnew\n\nDiffer at:*** fixture-a\n--- fixture-b\n! changed\n\n" ((:find "diff") (:call "/fixture/bin/diff" nil t nil "-c" "alpha\nold\n" "alpha\nnew\n")) t)"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn explain_equality_selects_equal_diff_and_fallback_paths() {
    let elisp_form = r##"
(let (calls)
  (cl-letf
      (((symbol-function
         'assess--explainer-diff-string=)
        (lambda (a b)
          (push (list :diff a b) calls)
          "DIFF"))
       ((symbol-function
         'assess--explainer-simple-string=)
        (lambda (a b)
          (push (list :simple a b) calls)
          "SIMPLE")))
    (list
     (cl-letf
         (((symbol-function 'executable-find)
           (lambda (_) "/fixture/diff")))
       (assess-explain= "same" "same"))
     (cl-letf
         (((symbol-function 'executable-find)
           (lambda (_) "/fixture/diff")))
       (assess-explain= "left" "right"))
     (cl-letf
         (((symbol-function 'executable-find)
           (lambda (_) nil)))
       (assess-explain= "up" "down"))
     (nreverse calls))))
"##;
    let expect: Expect =
        expect![[r#"OK (t "DIFF" "SIMPLE" ((:diff "left" "right") (:simple "up" "down")))"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn silent_writer_persists_current_buffer_without_changing_messages_or_visit_state() {
    let elisp_form = r##"
(let ((path
       (assess-test-path
        "writer/output.txt"))
      before-message
      after-message)
  (make-directory (file-name-directory path) t)
  (with-temp-buffer
    (insert "written\nwithout chatter\n")
    (setq before-message
          (current-message))
    (assess--write-file-silently path)
    (setq after-message
          (current-message))
    (list
     (assess-test-read-file path)
     (equal before-message after-message)
     buffer-file-name
     (buffer-modified-p))))
"##;
    let expect: Expect = expect![[r#"OK ("written\nwithout chatter\n" t nil t)"#]];
    assert_assess_parity(elisp_form, expect);
}

#[test]
fn ert_pretty_print_advice_temporarily_disables_newline_escaping() {
    let elisp_form = r##"
(let ((pp-escape-newlines 'outer)
      observed)
  (list
   (assess--ert-pp-with-indentation-and-newline
    (lambda (object)
      (setq observed pp-escape-newlines)
      (list object pp-escape-newlines))
    "alpha\nbeta")
   observed
   pp-escape-newlines
   (advice-member-p
    #'assess--ert-pp-with-indentation-and-newline
    'ert--pp-with-indentation-and-newline)))
"##;
    let expect: Expect = expect![[
        r#"OK (("alpha\nbeta" nil) nil outer #[128 "������\3#��" [assess--ert-pp-with-indentation-and-newline #[(object) ((let ((pp-escape-newlines t) (print-escape-control-characters t)) (pp object (current-buffer)))) (cl-struct-ert--stats-tags cl-struct-ert--test-execution-info-tags cl-struct-ert-test-aborted-with-non-local-exit-tags cl-struct-ert-test-skipped-tags cl-struct-ert-test-failed-tags cl-struct-ert-test-quit-tags cl-struct-ert-test-result-with-condition-tags cl-struct-ert-test-passed-tags cl-struct-ert-test-result-tags cl-struct-ert-test-tags t) nil "Pretty-print OBJECT, indenting it to the current column of point.\nEnsures a final newline is inserted."] :around nil apply] 5 advice])"#
    ]];
    assert_assess_parity(elisp_form, expect);
}

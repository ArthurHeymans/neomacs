use expect_test::expect;

use super::assert_abgaben_parity;

#[test]
fn abgaben_public_surface_dependencies_and_command_classification_match_the_pin() {
    let elisp_form = r##"(list
               (featurep 'abgaben)
               (mapcar
                (lambda (feature)
                  (featurep feature))
                '(pdf-annot f s mu4e))
               (mapcar
                #'fboundp
                '(abgaben--get-group
                  abgaben--get-week
                  abgaben-capture-submission
                  abgaben--maybe-unzip
                  abgaben-get-file-at-heading
                  abgaben-export-pdf-annot-to-org
                  abgaben-matches-in-buffer
                  abgaben--construct-email-body
                  abgaben-prepare-reply))
               (mapcar
                #'commandp
                '(abgaben--get-group
                  abgaben--get-week
                  abgaben-capture-submission
                  abgaben--maybe-unzip
                  abgaben-get-file-at-heading
                  abgaben-export-pdf-annot-to-org
                  abgaben-matches-in-buffer
                  abgaben--construct-email-body
                  abgaben-prepare-reply)))"##;
    let expect = expect!["OK (t (t t t t) (t t t t t t t t t) (nil nil nil nil nil t nil nil t))"];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_defaults_and_custom_metadata_match_the_pin() {
    let elisp_form = r##"(list
               abgaben-pdf-tools-org-non-exportable-types
               abgaben-root-folder
               abgaben-org-file
               abgaben-heading
               abgaben-points-re
               abgaben-all-groups
               abgaben-all-weeks
               abgaben-points-heading
               abgaben-points-overall
               abgaben--curr-week
               abgaben--curr-group
               (mapcar
                (lambda (variable)
                  (list
                   variable
                   (get variable 'custom-group)
                   (get variable 'custom-type)
                   (eval
                    (car
                     (get variable 'standard-value)))))
                '(abgaben-root-folder
                  abgaben-org-file
                  abgaben-heading
                  abgaben-points-re
                  abgaben-all-groups
                  abgaben-all-weeks
                  abgaben-points-heading
                  abgaben-points-overall)))"##;
    let expect = expect![[
        r#"OK ((link) "[ORACLE-SANDBOX]/$HOME/abgaben/" "[ORACLE-SANDBOX]/$HOME/abgaben/abgaben.org" "Abgaben" "assignment [0-9.]*: ?\\([0-9.]*\\)/\\([0-9.]*\\)" #1=("group1" "group2") ("01" "02" "03" "04" "05" "06" "07" "08" "09" "10" "11" "12" "13" "14") "your points" "overall" "01" "group1" ((abgaben-root-folder nil (string) "[ORACLE-SANDBOX]/$HOME/abgaben/") (abgaben-org-file nil (string) "[ORACLE-SANDBOX]/$HOME/abgaben/abgaben.org") (abgaben-heading nil (string) "Abgaben") (abgaben-points-re nil (regexp) "assignment [0-9.]*: ?\\([0-9.]*\\)/\\([0-9.]*\\)") (abgaben-all-groups nil (repeat string) #1#) (abgaben-all-weeks nil (repeat string) ("01" "02" "03" "04" "05" "06" "07" "08" "09" "10" "11" "12" "13" "14")) (abgaben-points-heading nil string "your points") (abgaben-points-overall nil string "overall")))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_group_and_week_readers_preserve_exact_completion_contract_and_new_defaults() {
    let elisp_form = r##"(let ((abgaben-all-groups '("red" "blue"))
                    (abgaben-all-weeks '("01" "02"))
                    (abgaben--curr-group "red")
                    (abgaben--curr-week "01")
                    (answers '("blue" "02"))
                    events)
               (cl-letf
                   (((symbol-function 'completing-read)
                     (lambda (&rest arguments)
                       (push (cons 'complete arguments) events)
                       (pop answers))))
                 (list
                  (abgaben--get-group)
                  abgaben--curr-group
                  (abgaben--get-week)
                  abgaben--curr-week
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK ("blue" "blue" "02" "02" ((complete "Which group? " ("red" "blue") nil t nil nil "red") (complete "Which week? " ("01" "02") nil t nil nil "01")))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

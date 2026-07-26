use expect_test::expect;

use super::{assert_academic_phrases_parity, assert_academic_phrases_signal_parity};

#[test]
fn academic_phrases_insert_by_section_selects_every_exact_category_range() {
    let elisp_form = r##"(let (captured)
               (cl-letf
                   (((symbol-function
                      'academic-phrases--insert)
                     (lambda (phrases)
                       (let ((keys
                              (ht-keys
                               phrases)))
                         (setq
                          keys
                          (sort
                           keys
                           (lambda (left right)
                             (<
                              (string-to-number
                               (substring
                                (symbol-name
                                 left)
                                4))
                              (string-to-number
                               (substring
                                (symbol-name
                                 right)
                                4))))))
                         (setq
                          captured
                          keys)
                         (list
                          'inserted
                          (length
                           keys))))))
                 (mapcar
                  (lambda (section)
                    (let ((result
                           (academic-phrases--insert-by-section
                            section)))
                      (list
                       section
                       result
                       captured)))
                  '(:abstract
                    :intro
                    :methods
                    :results
                    :discussion
                    :conclusion
                    :acknowledgments
                    :unknown
                    nil))))"##;
    let expect = expect![
        "OK ((:abstract (inserted 4) (:cat1 :cat2 :cat4 :cat5)) (:intro (inserted 16) (:cat1 :cat2 :cat3 :cat4 :cat5 :cat6 :cat7 :cat8 :cat9 :cat10 :cat11 :cat12 :cat13 :cat14 :cat15 :cat16)) (:methods (inserted 14) (:cat17 :cat18 :cat19 :cat20 :cat21 :cat22 :cat23 :cat24 :cat25 :cat26 :cat27 :cat28 :cat29 :cat30)) (:results (inserted 12) (:cat29 :cat30 :cat31 :cat32 :cat33 :cat34 :cat35 :cat36 :cat37 :cat38 :cat39 :cat40)) (:discussion (inserted 11) (:cat35 :cat36 :cat37 :cat38 :cat39 :cat40 :cat41 :cat42 :cat43 :cat44 :cat45)) (:conclusion (inserted 7) (:cat45 :cat46 :cat47 :cat48 :cat49 :cat50 :cat51)) (:acknowledgments (inserted 1) (:cat52)) (:unknown (inserted 57) (:cat1 :cat2 :cat3 :cat4 :cat5 :cat6 :cat7 :cat8 :cat9 :cat10 :cat11 :cat12 :cat13 :cat14 :cat15 :cat16 :cat17 :cat18 :cat19 :cat20 :cat21 :cat22 :cat23 :cat24 :cat25 :cat26 :cat27 :cat28 :cat29 :cat30 :cat31 :cat32 :cat33 :cat34 :cat35 :cat36 :cat37 :cat38 :cat39 :cat40 :cat41 :cat42 :cat43 :cat44 :cat45 :cat46 :cat47 :cat48 :cat49 :cat50 :cat51 :cat52 :cat53 :cat54 :cat55 :cat56 :cat57)) (nil (inserted 57) (:cat1 :cat2 :cat3 :cat4 :cat5 :cat6 :cat7 :cat8 :cat9 :cat10 :cat11 :cat12 :cat13 :cat14 :cat15 :cat16 :cat17 :cat18 :cat19 :cat20 :cat21 :cat22 :cat23 :cat24 :cat25 :cat26 :cat27 :cat28 :cat29 :cat30 :cat31 :cat32 :cat33 :cat34 :cat35 :cat36 :cat37 :cat38 :cat39 :cat40 :cat41 :cat42 :cat43 :cat44 :cat45 :cat46 :cat47 :cat48 :cat49 :cat50 :cat51 :cat52 :cat53 :cat54 :cat55 :cat56 :cat57)))"
    ];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_review_section_surfaces_the_unprovided_concatenate_function() {
    let elisp_form = r##"(academic-phrases--insert-by-section
              :review)"##;
    let expect = expect!["ERR (void-function concatenate)"];

    assert_academic_phrases_signal_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_section_command_surfaces_review_selection_concatenate_failure() {
    let elisp_form = r##"(cl-letf
              (((symbol-function
                 'completing-read)
                (lambda (&rest _)
                  "Literature Review")))
              (academic-phrases-by-section))"##;
    let expect = expect!["ERR (void-function concatenate)"];

    assert_academic_phrases_signal_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_insert_by_section_accepts_an_explicit_live_phrase_table() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'academic-phrases--insert)
                     (lambda (phrases)
                       (push
                        (list
                         (eq
                          phrases
                          academic-phrases--all-phrases)
                         (hash-table-count
                          phrases)
                         (sort
                          (ht-keys
                           phrases)
                          (lambda (left right)
                            (string<
                             (symbol-name
                              left)
                             (symbol-name
                              right)))))
                        calls)
                       'inserted)))
                 (list
                  (academic-phrases--insert-by-section
                   :abstract
                   academic-phrases--all-phrases)
                  (academic-phrases--insert-by-section
                   :acknowledgments
                   academic-phrases--all-phrases)
                  (nreverse
                   calls))))"##;
    let expect =
        expect!["OK (inserted inserted ((nil 4 (:cat1 :cat2 :cat4 :cat5)) (nil 1 (:cat52))))"];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_section_command_maps_every_prompt_choice_and_exact_completion_contract() {
    let elisp_form = r##"(let ((responses
                    '("Abstract"
                      "Introduction"
                      "Literature Review"
                      "Methods"
                      "Results"
                      "Discussion"
                      "Conclusions"
                      "Acknowledgements"
                      "Not a section"
                      "Abstract"))
                   calls
                   sections)
               (cl-letf
                   (((symbol-function
                      'completing-read)
                     (lambda (prompt collection
                              &optional predicate require-match
                              initial-input history default
                              inherit-input-method)
                       (push
                        (list
                         prompt
                         (copy-tree
                          collection)
                         predicate
                         require-match
                         initial-input
                         history
                         default
                         inherit-input-method)
                        calls)
                       (pop
                        responses)))
                    ((symbol-function
                      'academic-phrases--insert-by-section)
                     (lambda (section)
                       (push
                        section
                        sections)
                       section)))
                 (let ((direct
                        (mapcar
                         (lambda (_)
                           (academic-phrases-by-section))
                         (number-sequence
                          1
                          9)))
                       (interactive
                        (call-interactively
                         #'academic-phrases-by-section)))
                   (list
                    direct
                    interactive
                    (nreverse
                     sections)
                    (nreverse
                     calls)
                    responses
                    (interactive-form
                     'academic-phrases-by-section)))))"##;
    let expect = expect![[
        r#"OK ((:abstract :intro :review :methods :results :discussion :conclusion :acknowledgments nil) :abstract (:abstract :intro :review :methods :results :discussion :conclusion :acknowledgments nil :abstract) (("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil) ("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil) ("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil) ("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil) ("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil) ("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil) ("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil) ("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil) ("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil) ("Choose a section: " (("Abstract" . :abstract) ("Introduction" . :intro) ("Literature Review" . :review) ("Methods" . :methods) ("Results" . :results) ("Discussion" . :discussion) ("Conclusions" . :conclusion) ("Acknowledgements" . :acknowledgments)) nil t nil nil nil nil)) nil (interactive nil))"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

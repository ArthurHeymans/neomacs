use expect_test::expect;

use super::assert_abgaben_parity;

#[test]
fn abgaben_export_annotations_sorts_filters_inserts_contents_and_sums_points() {
    let elisp_form = r##"(progn
               (require 'org)
               (let ((abgaben-points-heading "Points")
                     (abgaben-points-overall "Total")
                     (abgaben-points-re
                      "assignment [0-9.]*: ?\\([0-9.]*\\)/\\([0-9.]*\\)")
                     (abgaben-pdf-tools-org-non-exportable-types
                      '(link))
                     events)
                 (with-temp-buffer
                   (org-mode)
                   (insert
                    "* [[file:/work/paper.pdf][Paper]] Email: [[mu4e:msgid:id][Mail]]\n"
                    "** stale export\n")
                   (goto-char (point-min))
                   (cl-letf
                       (((symbol-function 'pdf-info-getannots)
                         (lambda (&rest arguments)
                           (push
                            (cons 'getannots arguments)
                            events)
                           '((:id third
                              :type text
                              :contents "assignment 3: 2.5/3")
                             (:id hidden
                              :type link
                              :contents "assignment 9: 99/99")
                             (:id first
                              :type text
                              :contents "assignment 1: 1/2")
                             (:id second
                              :type text
                              :contents "plain note"))))
                        ((symbol-function
                          'pdf-annot-compare-annotations)
                         (lambda (left right)
                           (let ((order
                                  '((first . 1)
                                    (second . 2)
                                    (third . 3)
                                    (hidden . 4))))
                             (<
                              (cdr
                               (assq
                                (plist-get left :id)
                                order))
                              (cdr
                               (assq
                                (plist-get right :id)
                                order))))))
                        ((symbol-function 'pdf-annot-get-type)
                         (lambda (annotation)
                           (plist-get annotation :type)))
                        ((symbol-function 'pdf-annot-get-id)
                         (lambda (annotation)
                           (plist-get annotation :id)))
                        ((symbol-function 'pdf-annot-get)
                         (lambda (annotation property)
                           (and
                            (eq property 'contents)
                            (plist-get annotation :contents)))))
                     (list
                      (abgaben-export-pdf-annot-to-org)
                      (buffer-string)
                      (point)
                      (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (nil "* [[file:/work/paper.pdf][Paper]] Email: [[mu4e:msgid:id][Mail]]\n** Points\nassignment 1: 1/2\nassignment 3: 2.5/3\nTotal: 3.5/5 \n** first\nassignment 1: 1/2\n** second\nplain note\n** third\nassignment 3: 2.5/3\n" 1 ((getannots nil "/work/paper.pdf")))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_export_annotations_handles_no_score_matches_with_zero_totals() {
    let elisp_form = r##"(progn
               (require 'org)
               (with-temp-buffer
                 (org-mode)
                 (insert "* [[file:/work/paper.pdf][Paper]]\n")
                 (goto-char (point-min))
                 (cl-letf
                     (((symbol-function 'pdf-info-getannots)
                       (lambda (&rest _)
                         '((:id note
                            :type text
                            :contents "no score"))))
                      ((symbol-function
                        'pdf-annot-compare-annotations)
                       (lambda (&rest _)
                         nil))
                      ((symbol-function 'pdf-annot-get-type)
                       (lambda (annotation)
                         (plist-get annotation :type)))
                      ((symbol-function 'pdf-annot-get-id)
                       (lambda (annotation)
                         (plist-get annotation :id)))
                      ((symbol-function 'pdf-annot-get)
                       (lambda (annotation property)
                         (and
                          (eq property 'contents)
                          (plist-get annotation :contents)))))
                   (list
                    (abgaben-export-pdf-annot-to-org)
                    (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (nil "* [[file:/work/paper.pdf][Paper]]\n** your points\noverall: 0/0 \n** note\nno score\n")"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_construct_email_body_copies_subtree_removes_heading_and_appends_attachment() {
    let elisp_form = r##"(progn
               (require 'org)
               (let ((kill-ring nil))
                 (with-temp-buffer
                   (org-mode)
                   (insert
                    "* Root\n"
                    "** [[file:/work/My%20Paper.pdf][Paper]] Email: [[mu4e:msgid:id][Mail]]\n"
                    "*** Feedback\n"
                    "Good work.\n"
                    "*** Points\n"
                    "Total: 8/10\n"
                    "** Other\n")
                   (goto-char (point-min))
                   (search-forward "Paper")
                   (beginning-of-line)
                   (list
                    (abgaben--construct-email-body)
                    (car kill-ring)
                    (point)
                    (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (nil "*** Feedback\nGood work.\n*** Points\nTotal: 8/10\n<#part type=\"application/pdf\" filename=\"/work/My%20Paper.pdf\" disposition=attachment><#/part>" 8 "* Root\n** [[file:/work/My%20Paper.pdf][Paper]] Email: [[mu4e:msgid:id][Mail]]\n*** Feedback\nGood work.\n*** Points\nTotal: 8/10\n** Other\n")"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

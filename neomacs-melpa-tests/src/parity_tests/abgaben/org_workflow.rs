use expect_test::expect;

use super::assert_abgaben_parity;

#[test]
fn abgaben_get_file_at_heading_returns_first_file_link_and_unescapes_its_path() {
    let elisp_form = r##"(progn
               (require 'org)
               (with-temp-buffer
                 (org-mode)
                 (insert
                  "* Root\n"
                  "** [[https://example.invalid][web]] [[file:/work/My%20Paper.pdf][paper]]\n"
                  "Body [[file:/work/second.pdf][second]]\n"
                  "** No file\n"
                  "Only [[https://example.invalid][web]].\n")
                 (goto-char (point-min))
                 (search-forward "My%20Paper")
                 (beginning-of-line)
                 (let ((first
                        (list
                         (abgaben-get-file-at-heading)
                         (point))))
                   (search-forward "No file")
                   (beginning-of-line)
                   (list
                    first
                    (abgaben-get-file-at-heading)
                    (point)))))"##;
    let expect = expect![[r#"OK (("/work/My%20Paper.pdf" 8) nil 120)"#]];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_matches_in_buffer_preserves_order_point_and_callers_match_data() {
    let elisp_form = r##"(let ((other
                    (generate-new-buffer
                     " *abgaben-match-source*")))
               (unwind-protect
                   (progn
                     (with-current-buffer other
                       (insert
                        "assignment 1: 2.5/3\n"
                        "noise\n"
                        "assignment 2: 0/4\n"
                        "assignment 3: 3/3 twice assignment 4: 1/1\n"))
                     (with-temp-buffer
                       (insert "caller")
                       (goto-char 4)
                       (string-match "\\(all\\)" "caller")
                       (let ((before
                              (list
                               (point)
                               (match-string 1 "caller"))))
                         (list
                          (abgaben-matches-in-buffer
                           "assignment [0-9]+: [0-9.]+/[0-9.]+"
                           other)
                          before
                          (list
                           (point)
                           (match-string 1 "caller"))))))
                 (when (buffer-live-p other)
                   (kill-buffer other))))"##;
    let expect = expect![[
        r#"OK (("assignment 1: 2.5/3" "assignment 2: 0/4" "assignment 3: 3/3" "assignment 4: 1/1") (4 "all") (4 "all"))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_capture_submission_inserts_under_existing_week_with_exact_links_and_order() {
    let elisp_form = r##"(progn
               (require 'org)
               (let ((target
                      (generate-new-buffer
                       " *abgaben-capture-existing*"))
                     (abgaben-root-folder "/grading")
                     (abgaben-org-file "/notes/abgaben.org")
                     (abgaben-heading "Abgaben")
                     (abgaben--curr-group "old-group")
                     (abgaben--curr-week "old-week")
                     events)
                 (unwind-protect
                     (progn
                       (with-current-buffer target
                         (org-mode)
                         (insert
                          "* Abgaben\n"
                          "** red\n"
                          "*** 03\n"
                          "**** Previous\n"))
                       (cl-letf
                           (((symbol-function 'abgaben--get-group)
                             (lambda ()
                               (push '(get-group) events)
                               (setq abgaben--curr-group "red")))
                            ((symbol-function 'abgaben--get-week)
                             (lambda ()
                               (push '(get-week) events)
                               (setq abgaben--curr-week "03")))
                            ((symbol-function 'mu4e~view-get-attach)
                             (lambda (msg attnum)
                               (push
                                (list 'get-attach msg attnum)
                                events)
                               '(:name "submission.zip")))
                            ((symbol-function 'make-directory)
                             (lambda (&rest arguments)
                               (push
                                (cons 'make-directory arguments)
                                events)))
                            ((symbol-function
                              'mu4e-view-save-attachment-single)
                             (lambda (&rest arguments)
                               (push
                                (cons 'save-attachment arguments)
                                events)))
                            ((symbol-function 'find-file)
                             (lambda (file)
                               (push (list 'find-file file) events)
                               (set-buffer target)))
                            ((symbol-function 'abgaben--maybe-unzip)
                             (lambda (directory name)
                               (push
                                (list 'maybe-unzip directory name)
                                events)
                               "submission")))
                         (abgaben-capture-submission
                          '(:message-id "id-1"
                            :subject "Assignment α")
                          2)
                         (list
                          (with-current-buffer target
                            (buffer-string))
                          abgaben--curr-group
                          abgaben--curr-week
                          (nreverse events))))
                   (when (buffer-live-p target)
                     (kill-buffer target)))))"##;
    let expect = expect![[
        r#"OK ("* Abgaben\n** red\n*** 03\n**** [[file:/grading/red/03/submission][submission.zip]] Email: [[mu4e:msgid:id-1][Assignment α]]\n**** Previous\n" "red" "03" ((get-group) (get-week) (get-attach #1=(:message-id "id-1" :subject "Assignment α") 2) (make-directory "/grading/red/03" t) (save-attachment #1# 2) (find-file "/notes/abgaben.org") (maybe-unzip "/grading/red/03" "submission.zip")))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_capture_submission_creates_missing_week_and_uses_fallback_mail_fields() {
    let elisp_form = r##"(progn
               (require 'org)
               (let ((target
                      (generate-new-buffer
                       " *abgaben-capture-new*"))
                     (abgaben-root-folder "/grading/")
                     (abgaben-org-file "/notes/abgaben.org")
                     (abgaben-heading "Root")
                     events)
                 (unwind-protect
                     (progn
                       (with-current-buffer target
                         (org-mode)
                         (insert "* Root\n** blue\n"))
                       (cl-letf
                           (((symbol-function 'abgaben--get-group)
                             (lambda ()
                               (setq abgaben--curr-group "blue")))
                            ((symbol-function 'abgaben--get-week)
                             (lambda ()
                               (setq abgaben--curr-week "14")))
                            ((symbol-function 'mu4e~view-get-attach)
                             (lambda (&rest _)
                               '(:name "answer.pdf")))
                            ((symbol-function 'make-directory)
                             (lambda (&rest arguments)
                               (push
                                (cons 'make-directory arguments)
                                events)))
                            ((symbol-function
                              'mu4e-view-save-attachment-single)
                             (lambda (&rest arguments)
                               (push
                                (cons 'save-attachment arguments)
                                events)))
                            ((symbol-function 'find-file)
                             (lambda (file)
                               (push (list 'find-file file) events)
                               (set-buffer target)))
                            ((symbol-function 'abgaben--maybe-unzip)
                             (lambda (directory name)
                               (push
                                (list 'maybe-unzip directory name)
                                events)
                               name)))
                         (abgaben-capture-submission nil 7)
                         (list
                          (with-current-buffer target
                            (buffer-string))
                          (nreverse events))))
                   (when (buffer-live-p target)
                     (kill-buffer target)))))"##;
    let expect = expect![[
        r#"OK ("* Root\n** blue\n*** 14\n**** [[file:/grading/blue/14/answer.pdf][answer.pdf]] Email: [[mu4e:msgid:<none>][<none>]]\n" ((make-directory "/grading/blue/14" t) (save-attachment nil 7) (find-file "/notes/abgaben.org") (maybe-unzip "/grading/blue/14" "answer.pdf")))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

#[test]
fn abgaben_prepare_reply_constructs_body_then_opens_the_mail_link_at_point() {
    let elisp_form = r##"(progn
               (require 'org)
               (let (events)
                 (with-temp-buffer
                   (org-mode)
                   (insert
                    "* Assignment Email: [[mu4e:msgid:id-9][Subject]]\n"
                    "Body\n")
                   (goto-char 4)
                   (cl-letf
                       (((symbol-function
                          'abgaben--construct-email-body)
                         (lambda ()
                           (push
                            (list 'construct (point))
                            events)
                           'constructed))
                        ((symbol-function 'org-open-at-point)
                         (lambda ()
                           (push
                            (list
                             'open
                             (point)
                             (org-element-context))
                            events)
                           'opened)))
                     (list
                      (abgaben-prepare-reply)
                      (point)
                      (nreverse events))))))"##;
    let expect = expect![[
        r#"OK (opened 4 ((construct 4) (open 23 (link (:standard-properties [21 nil 40 47 49 0 nil nil nil nil nil nil nil nil (:buffer nil) nil nil (headline (:standard-properties [1 1 50 55 55 0 (:title) first-section element t nil 52 55 1 (:buffer nil) [org-element-deferred org-element--headline-deferred nil t] nil (org-data (:standard-properties [1 1 1 55 55 0 nil org-data nil t nil 3 55 nil (:buffer nil) [org-element-deferred org-element--get-global-node-properties nil t] nil nil] :pre-blank 0 :path nil))] :pre-blank 0 :raw-value #1=[org-element-deferred org-element--headline-parse-title (t) t] :title #1# :level #1# :priority #1# :tags #1# :todo-keyword #1# :todo-type #1# :footnote-section-p #1# :archivedp #1# :commentedp #1#))] :type "fuzzy" :type-explicit-p nil :path "mu4e:msgid:id-9" :format bracket :raw-link "mu4e:msgid:id-9" :application nil :search-option nil)))))"#
    ]];

    assert_abgaben_parity(elisp_form, expect);
}

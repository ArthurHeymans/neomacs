use expect_test::expect;

use super::assert_all_the_icons_completion_parity;

#[test]
fn non_affixation_metadata_properties_delegate_once_with_original_arguments() {
    let elisp_form = r##"
(let (calls)
  (cl-labels
      ((original
        (metadata property)
        (push (list metadata property) calls)
        (alist-get property metadata)))
    (list
     (all-the-icons-completion-completion-metadata-get
      #'original
      '((category . file)
        (display-sort-function . identity))
      'category)
     (all-the-icons-completion-completion-metadata-get
      #'original
      '((category . file)
        (display-sort-function . identity))
      'display-sort-function)
     (nreverse calls))))
"##;
    let expect = expect![
        "OK (file identity ((((category . file) (display-sort-function . identity)) category) (((category . file) (display-sort-function . identity)) display-sort-function)))"
    ];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn category_without_existing_affixation_builds_icon_prefix_triples() {
    let elisp_form = r##"
(cl-defmethod all-the-icons-completion-get-icon
  (candidate (_category (eql test-category)))
  (format "<icon:%s>" candidate))
(let* ((metadata '((category . test-category)))
       (original
        (lambda (metadata property)
          (alist-get property metadata)))
       (affix
        (all-the-icons-completion-completion-metadata-get
         original metadata 'affixation-function)))
  (list
   (functionp affix)
   (funcall affix '("alpha" "βeta" ""))))
"##;
    let expect = expect![[
        r#"OK (t (("alpha" "<icon:alpha>" "") ("βeta" "<icon:βeta>" "") ("" "<icon:>" "")))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn existing_affixation_keeps_candidate_and_suffix_and_prepends_icon_to_prefix() {
    let elisp_form = r##"
(cl-defmethod all-the-icons-completion-get-icon
  (candidate (_category (eql test-affix)))
  (format "<%s>" (upcase candidate)))
(let* ((metadata
        '((category . test-affix)
          (affixation-function
           . (lambda (candidates)
               (mapcar
                (lambda (candidate)
                  (list
                   candidate
                   (concat "[" candidate "]")
                   (concat " — suffix:" candidate)))
                candidates)))))
       (original
        (lambda (metadata property)
          (alist-get property metadata)))
       (affix
        (all-the-icons-completion-completion-metadata-get
         original metadata 'affixation-function)))
  (funcall affix '("alpha" "Zażółć")))
"##;
    let expect = expect![[
        r#"OK (("alpha" "<ALPHA>[alpha]" " — suffix:alpha") ("Zażółć" "<ZAŻÓŁĆ>[Zażółć]" " — suffix:Zażółć"))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn annotation_function_is_adapted_to_affixation_and_called_once_per_candidate() {
    let elisp_form = r##"
(cl-defmethod all-the-icons-completion-get-icon
  (candidate (_category (eql test-annotation)))
  (format "<icon:%s>" candidate))
(let ((annotation-calls nil))
  (let* ((metadata
          `((category . test-annotation)
            (annotation-function
             . ,(lambda (candidate)
                  (push candidate annotation-calls)
                  (format " (%d chars)" (length candidate))))))
         (original
          (lambda (metadata property)
            (alist-get property metadata)))
         (affix
          (all-the-icons-completion-completion-metadata-get
           original metadata 'affixation-function)))
    (list
     (funcall affix '("one" "four" "六"))
     (nreverse annotation-calls))))
"##;
    let expect = expect![[
        r#"OK ((("one" "<icon:one>" " (3 chars)") ("four" "<icon:four>" " (4 chars)") ("六" "<icon:六>" " (1 chars)")) ("one" "four" "六"))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn native_affixation_takes_precedence_and_annotation_is_never_evaluated() {
    let elisp_form = r##"
(cl-defmethod all-the-icons-completion-get-icon
  (candidate (_category (eql test-precedence)))
  (format "<%s>" candidate))
(let ((affix-calls 0)
      (annotation-calls 0))
  (let* ((metadata
          `((category . test-precedence)
            (affixation-function
             . ,(lambda (candidates)
                  (setq affix-calls (1+ affix-calls))
                  (mapcar
                   (lambda (candidate)
                     (list candidate "native:" ":done"))
                   candidates)))
            (annotation-function
             . ,(lambda (_candidate)
                  (setq annotation-calls
                        (1+ annotation-calls))
                  "unexpected"))))
         (original
          (lambda (metadata property)
            (alist-get property metadata)))
         (affix
          (all-the-icons-completion-completion-metadata-get
           original metadata 'affixation-function)))
    (list
     (funcall affix '("one" "two"))
     affix-calls
     annotation-calls)))
"##;
    let expect =
        expect![[r#"OK ((("one" "<one>native:" ":done") ("two" "<two>native:" ":done")) 1 0)"#]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn metadata_without_category_preserves_existing_affixation_or_annotation_adapter() {
    let elisp_form = r##"
(let* ((native
        (lambda (candidates)
          (mapcar
           (lambda (candidate)
             (list candidate "native" "suffix"))
           candidates)))
       (annotate
        (lambda (candidate)
          (concat " note:" candidate)))
       (original
        (lambda (metadata property)
          (alist-get property metadata)))
       (native-result
        (all-the-icons-completion-completion-metadata-get
         original
         `((affixation-function . ,native))
         'affixation-function))
       (annotation-result
        (all-the-icons-completion-completion-metadata-get
         original
         `((annotation-function . ,annotate))
         'affixation-function))
       (empty-result
        (all-the-icons-completion-completion-metadata-get
         original nil 'affixation-function)))
  (list
   (eq native native-result)
   (funcall native-result '("x"))
   (funcall annotation-result '("x" "y"))
   empty-result))
"##;
    let expect = expect![[
        r#"OK (t (("x" "native" "suffix")) (("x" "" " note:x") ("y" "" " note:y")) nil)"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn multi_category_without_affixation_uses_each_candidates_original_value_and_category() {
    let elisp_form = r##"
(cl-defmethod all-the-icons-completion-get-icon
  (candidate (_category (eql service)))
  (format "<service:%s>" candidate))
(cl-defmethod all-the-icons-completion-get-icon
  (candidate (_category (eql region)))
  (format "<region:%s>" candidate))
(let* ((first
        (propertize
         "API (production)"
         'multi-category '(service . "api")))
       (second
        (propertize
         "EU West"
         'multi-category '(region . "eu-west-1")))
       (metadata '((category . multi-category)))
       (original
        (lambda (metadata property)
          (alist-get property metadata)))
       (affix
        (all-the-icons-completion-completion-metadata-get
         original metadata 'affixation-function)))
  (funcall affix (list first second)))
"##;
    let expect = expect![[
        r#"OK ((#("API (production)" 0 16 (multi-category (service . "api"))) "<service:api>" "") (#("EU West" 0 7 (multi-category (region . "eu-west-1"))) "<region:eu-west-1>" ""))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn multi_category_existing_affixation_preserves_text_properties_prefix_and_suffix() {
    let elisp_form = r##"
(cl-defmethod all-the-icons-completion-get-icon
  (candidate (_category (eql account)))
  (format "<account:%s>" candidate))
(let* ((candidate
        (propertize
         "Primary account"
         'multi-category '(account . "acct-42")
         'audit-id 42))
       (metadata
        `((category . multi-category)
          (affixation-function
           . ,(lambda (candidates)
                (mapcar
                 (lambda (value)
                   (list value "existing:" " [active]"))
                 candidates)))))
       (original
        (lambda (metadata property)
          (alist-get property metadata)))
       (affix
        (all-the-icons-completion-completion-metadata-get
         original metadata 'affixation-function))
       (result (car (funcall affix (list candidate))))
       (returned (car result)))
  (list
   result
   (equal returned candidate)
   (get-text-property 0 'multi-category returned)
   (get-text-property 0 'audit-id returned)))
"##;
    let expect = expect![[
        r#"OK ((#("Primary account" 0 15 (multi-category (account . "acct-42") audit-id 42)) "<account:acct-42>existing:" " [active]") t (account . "acct-42") 42)"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn affixation_handles_empty_candidates_and_surfaces_malformed_triples_exactly() {
    let elisp_form = r##"
(cl-defmethod all-the-icons-completion-get-icon
  (candidate (_category (eql malformed-test)))
  (format "<%s>" candidate))
(let* ((original
        (lambda (metadata property)
          (alist-get property metadata)))
       (plain
        (all-the-icons-completion-completion-metadata-get
         original
         '((category . malformed-test))
         'affixation-function))
       (malformed
        (all-the-icons-completion-completion-metadata-get
         original
         '((category . malformed-test)
           (affixation-function
            . (lambda (_candidates)
                '(("alpha" "prefix-only")))))
         'affixation-function))
       outcome)
  (condition-case error-data
      (funcall malformed '("alpha"))
    (error
     (setq outcome
           (list
            (car error-data)
            (cadr error-data)))))
  (list
   (funcall plain nil)
   outcome))
"##;
    let expect =
        expect![[r#"OK (nil (error "No clause matching ‘(\"alpha\" \"prefix-only\")’"))"#]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

#[test]
fn real_completion_metadata_pipeline_adds_real_file_icons_without_replacing_candidates() {
    let elisp_form = r##"
(let* ((table
        (lambda (string predicate action)
          (if (eq action 'metadata)
              '(metadata (category . file))
            (complete-with-action
             action
             '("invoice.pdf" "src/lib.rs" "README.org")
             string
             predicate))))
       (metadata (completion-metadata "" table nil)))
  (unwind-protect
      (progn
        (all-the-icons-completion-mode 1)
        (let ((affix
               (completion-metadata-get
                metadata
                'affixation-function)))
          (list
           (all-completions "" table)
           (funcall
            affix
            '("invoice.pdf" "src/lib.rs" "README.org")))))
    (all-the-icons-completion-mode -1)))
"##;
    let expect = expect![[
        r#"OK (("invoice.pdf" "src/lib.rs" "README.org") (("invoice.pdf" #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #1=(:family #3="github-octicons" :height 1.2 :inherit all-the-icons-dred) face #1#)) "") ("src/lib.rs" #(" " 0 1 (rear-nonsticky t display (raise -0.24) font-lock-face #2=(:family "all-the-icons" :height 1.44 :inherit all-the-icons-maroon) face #2#)) "") ("README.org" #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #4=(:family #3# :height 1.2 :inherit all-the-icons-lcyan) face #4#)) "")))"#
    ]];
    assert_all_the_icons_completion_parity(elisp_form, expect);
}

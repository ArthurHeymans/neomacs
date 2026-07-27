use expect_test::expect;

use super::assert_all_the_icons_gnus_parity;

#[test]
fn set_format_assigns_complete_topic_group_summary_date_and_thread_tree_contract() {
    let elisp_form = r##"
(let ((result (all-the-icons-gnus--set-format)))
  (list
   result
   gnus-topic-line-format
   gnus-group-line-format
   gnus-summary-line-format
   gnus-user-date-format-alist
   gnus-sum-thread-tree-root
   gnus-sum-thread-tree-false-root
   gnus-sum-thread-tree-single-indent
   gnus-sum-thread-tree-leaf-with-other
   gnus-sum-thread-tree-vertical
   gnus-sum-thread-tree-single-leaf))
"##;
    let expect = expect![[
        r#"OK (#(" " 0 1 (rear-nonsticky t display #12=(raise 0.0) font-lock-face #1=(:family #2="Material Icons" :height 1.2) face #1#)) #("%i[  %(%{%n -- %A%}%) ]%v\n" 4 5 (rear-nonsticky t display (raise 0.0) font-lock-face #3=(:family #2# :height 1.2) face #3#)) #("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display (raise 0.0) font-lock-face #4=(:family #2# :height 1.2) face #4#)) #("%1{%U%R%z: %}%[%2{%&user-date;%}%]  %4{%-34,34n%} %3{ %}%(%1{%B%}%s%)\n" 35 36 (rear-nonsticky t display (raise 0.0) font-lock-face #5=(:family #2# :height 1.2) face #5#) 54 55 (rear-nonsticky t display (raise 0.0) font-lock-face #6=(:family #2# :height 1.2) face #6#)) ((t . #(" %Y-%m-%d %H:%M" 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #7=(:family #2# :height 1.2) face #7#)))) #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #8=(:family #2# :height 1.2) face #8#)) #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #9=(:family #2# :height 1.2) face #9#)) #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #10=(:family #2# :height 1.2) face #10#)) #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #11=(:family #2# :height 1.2) face #11#)) " " #(" " 0 1 (rear-nonsticky t display #12# font-lock-face #1# face #1#)))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn format_icons_come_from_real_all_the_icons_with_exact_glyph_properties() {
    let elisp_form = r##"
(all-the-icons-gnus--set-format)
(mapcar
 (lambda (case)
   (let* ((name (car case))
          (value (symbol-value (cdr case)))
          (icon-position
           (or
            (text-property-not-all
             0 (length value) 'face nil value)
            (text-property-not-all
             0 (length value) 'display nil value))))
     (list
      name
      value
      (substring-no-properties value)
      icon-position
      (and icon-position
           (text-properties-at icon-position value)))))
 '((topic . gnus-topic-line-format)
   (group . gnus-group-line-format)
   (summary . gnus-summary-line-format)
   (root . gnus-sum-thread-tree-root)
   (false-root . gnus-sum-thread-tree-false-root)
   (single-indent . gnus-sum-thread-tree-single-indent)
   (leaf-with-other . gnus-sum-thread-tree-leaf-with-other)
   (single-leaf . gnus-sum-thread-tree-single-leaf)))
"##;
    let expect = expect![[
        r#"OK ((topic #("%i[  %(%{%n -- %A%}%) ]%v\n" 4 5 (rear-nonsticky t display (raise 0.0) font-lock-face #1=(:family #3="Material Icons" :height 1.2) face #1#)) "%i[  %(%{%n -- %A%}%) ]%v\n" 4 (rear-nonsticky t display (raise 0.0) font-lock-face #2=(:family "Material Icons" :height 1.2) face #2#)) (group #("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display (raise 0.0) font-lock-face #4=(:family #3# :height 1.2) face #4#)) "%1M%1S%5y  : %(%-50,50G%)\n" 10 (rear-nonsticky t display (raise 0.0) font-lock-face #5=(:family "Material Icons" :height 1.2) face #5#)) (summary #("%1{%U%R%z: %}%[%2{%&user-date;%}%]  %4{%-34,34n%} %3{ %}%(%1{%B%}%s%)\n" 35 36 (rear-nonsticky t display (raise 0.0) font-lock-face #6=(:family #3# :height 1.2) face #6#) 54 55 (rear-nonsticky t display (raise 0.0) font-lock-face #7=(:family #3# :height 1.2) face #7#)) "%1{%U%R%z: %}%[%2{%&user-date;%}%]  %4{%-34,34n%} %3{ %}%(%1{%B%}%s%)\n" 35 (rear-nonsticky t display (raise 0.0) font-lock-face #8=(:family "Material Icons" :height 1.2) face #8#)) (root #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #9=(:family #3# :height 1.2) face #9#)) " " 0 (rear-nonsticky t display (raise 0.0) font-lock-face #10=(:family "Material Icons" :height 1.2) face #10#)) (false-root #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #11=(:family #3# :height 1.2) face #11#)) " " 0 (rear-nonsticky t display (raise 0.0) font-lock-face #12=(:family "Material Icons" :height 1.2) face #12#)) (single-indent #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #13=(:family #3# :height 1.2) face #13#)) " " 0 (rear-nonsticky t display (raise 0.0) font-lock-face #14=(:family "Material Icons" :height 1.2) face #14#)) (leaf-with-other #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #15=(:family #3# :height 1.2) face #15#)) " " 0 (rear-nonsticky t display (raise 0.0) font-lock-face #16=(:family "Material Icons" :height 1.2) face #16#)) (single-leaf #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #17=(:family #3# :height 1.2) face #17#)) " " 0 (rear-nonsticky t display (raise 0.0) font-lock-face #18=(:family "Material Icons" :height 1.2) face #18#)))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn set_format_overwrites_prior_user_values_and_returns_last_thread_leaf_assignment() {
    let elisp_form = r##"
(setq
 gnus-topic-line-format "custom-topic"
 gnus-group-line-format "custom-group"
 gnus-summary-line-format "custom-summary"
 gnus-user-date-format-alist '((t . "custom-date"))
 gnus-sum-thread-tree-root "custom-root"
 gnus-sum-thread-tree-false-root "custom-false"
 gnus-sum-thread-tree-single-indent "custom-indent"
 gnus-sum-thread-tree-leaf-with-other "custom-other"
 gnus-sum-thread-tree-vertical "custom-vertical"
 gnus-sum-thread-tree-single-leaf "custom-leaf")
(let ((result (all-the-icons-gnus--set-format)))
  (list
   result
   (equal result gnus-sum-thread-tree-single-leaf)
   gnus-topic-line-format
   gnus-group-line-format
   gnus-summary-line-format
   gnus-user-date-format-alist
   gnus-sum-thread-tree-root
   gnus-sum-thread-tree-false-root
   gnus-sum-thread-tree-single-indent
   gnus-sum-thread-tree-leaf-with-other
   gnus-sum-thread-tree-vertical
   gnus-sum-thread-tree-single-leaf))
"##;
    let expect = expect![[
        r#"OK (#(" " 0 1 (rear-nonsticky t display #12=(raise 0.0) font-lock-face #1=(:family #2="Material Icons" :height 1.2) face #1#)) t #("%i[  %(%{%n -- %A%}%) ]%v\n" 4 5 (rear-nonsticky t display (raise 0.0) font-lock-face #3=(:family #2# :height 1.2) face #3#)) #("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display (raise 0.0) font-lock-face #4=(:family #2# :height 1.2) face #4#)) #("%1{%U%R%z: %}%[%2{%&user-date;%}%]  %4{%-34,34n%} %3{ %}%(%1{%B%}%s%)\n" 35 36 (rear-nonsticky t display (raise 0.0) font-lock-face #5=(:family #2# :height 1.2) face #5#) 54 55 (rear-nonsticky t display (raise 0.0) font-lock-face #6=(:family #2# :height 1.2) face #6#)) ((t . #(" %Y-%m-%d %H:%M" 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #7=(:family #2# :height 1.2) face #7#)))) #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #8=(:family #2# :height 1.2) face #8#)) #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #9=(:family #2# :height 1.2) face #9#)) #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #10=(:family #2# :height 1.2) face #10#)) #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #11=(:family #2# :height 1.2) face #11#)) " " #(" " 0 1 (rear-nonsticky t display #12# font-lock-face #1# face #1#)))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn configured_user_date_format_runs_through_real_gnus_date_formatter_in_utc() {
    let elisp_form = r##"
(require 'gnus-sum)
(all-the-icons-gnus--set-format)
(let ((date "Mon, 01 Jan 2024 12:34:00 +0000"))
  (list
   gnus-user-date-format-alist
   (gnus-user-date date)
   (substring-no-properties (gnus-user-date date))
   (text-properties-at 0 (gnus-user-date date))))
"##;
    let expect = expect![[
        r#"OK (((t . #(" %Y-%m-%d %H:%M" 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #1=(:family "Material Icons" :height 1.2) face #1#)))) " 2024-01-01 12:34" " 2024-01-01 12:34" nil)"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn repeated_format_setup_is_value_and_text_property_idempotent() {
    let elisp_form = r##"
(all-the-icons-gnus--set-format)
(let ((first
       (mapcar
        (lambda (symbol)
          (copy-sequence (symbol-value symbol)))
        '(gnus-topic-line-format
          gnus-group-line-format
          gnus-summary-line-format
          gnus-user-date-format-alist
          gnus-sum-thread-tree-root
          gnus-sum-thread-tree-false-root
          gnus-sum-thread-tree-single-indent
          gnus-sum-thread-tree-leaf-with-other
          gnus-sum-thread-tree-vertical
          gnus-sum-thread-tree-single-leaf))))
  (all-the-icons-gnus--set-format)
  (let ((second
         (mapcar
          (lambda (symbol)
            (symbol-value symbol))
          '(gnus-topic-line-format
            gnus-group-line-format
            gnus-summary-line-format
            gnus-user-date-format-alist
            gnus-sum-thread-tree-root
            gnus-sum-thread-tree-false-root
            gnus-sum-thread-tree-single-indent
            gnus-sum-thread-tree-leaf-with-other
            gnus-sum-thread-tree-vertical
            gnus-sum-thread-tree-single-leaf))))
    (list
     (equal first second)
     first
     second)))
"##;
    let expect = expect![[
        r#"OK (t (#("%i[  %(%{%n -- %A%}%) ]%v\n" 4 5 (rear-nonsticky t display #12=(raise 0.0) font-lock-face #1=(:family #2="Material Icons" :height 1.2) face #1#)) #("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display #14=(raise 0.0) font-lock-face #3=(:family #2# :height 1.2) face #3#)) #("%1{%U%R%z: %}%[%2{%&user-date;%}%]  %4{%-34,34n%} %3{ %}%(%1{%B%}%s%)\n" 35 36 (rear-nonsticky t display #16=(raise 0.0) font-lock-face #4=(:family #2# :height 1.2) face #4#) 54 55 (rear-nonsticky t display #18=(raise 0.0) font-lock-face #5=(:family #2# :height 1.2) face #5#)) ((t . #(" %Y-%m-%d %H:%M" 0 1 (rear-nonsticky t display #20=(raise 0.0) font-lock-face #6=(:family #2# :height 1.2) face #6#)))) #(" " 0 1 (rear-nonsticky t display #22=(raise 0.0) font-lock-face #7=(:family #2# :height 1.2) face #7#)) #(" " 0 1 (rear-nonsticky t display #24=(raise 0.0) font-lock-face #8=(:family #2# :height 1.2) face #8#)) #(" " 0 1 (rear-nonsticky t display #26=(raise 0.0) font-lock-face #9=(:family #2# :height 1.2) face #9#)) #(" " 0 1 (rear-nonsticky t display #28=(raise 0.0) font-lock-face #10=(:family #2# :height 1.2) face #10#)) " " #(" " 0 1 (rear-nonsticky t display #30=(raise 0.0) font-lock-face #11=(:family #2# :height 1.2) face #11#))) (#("%i[  %(%{%n -- %A%}%) ]%v\n" 4 5 (rear-nonsticky t display #12# font-lock-face #13=(:family #2# :height 1.2) face #13#)) #("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display #14# font-lock-face #15=(:family #2# :height 1.2) face #15#)) #("%1{%U%R%z: %}%[%2{%&user-date;%}%]  %4{%-34,34n%} %3{ %}%(%1{%B%}%s%)\n" 35 36 (rear-nonsticky t display #16# font-lock-face #17=(:family #2# :height 1.2) face #17#) 54 55 (rear-nonsticky t display #18# font-lock-face #19=(:family #2# :height 1.2) face #19#)) ((t . #(" %Y-%m-%d %H:%M" 0 1 (rear-nonsticky t display #20# font-lock-face #21=(:family #2# :height 1.2) face #21#)))) #(" " 0 1 (rear-nonsticky t display #22# font-lock-face #23=(:family #2# :height 1.2) face #23#)) #(" " 0 1 (rear-nonsticky t display #24# font-lock-face #25=(:family #2# :height 1.2) face #25#)) #(" " 0 1 (rear-nonsticky t display #26# font-lock-face #27=(:family #2# :height 1.2) face #27#)) #(" " 0 1 (rear-nonsticky t display #28# font-lock-face #29=(:family #2# :height 1.2) face #29#)) " " #(" " 0 1 (rear-nonsticky t display #30# font-lock-face #31=(:family #2# :height 1.2) face #31#))))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn gnus_format_variables_are_global_and_visible_from_unrelated_buffers() {
    let elisp_form = r##"
(let ((first (generate-new-buffer " all-icons-gnus-first"))
      (second (generate-new-buffer " all-icons-gnus-second")))
  (unwind-protect
      (progn
        (all-the-icons-gnus--set-format)
        (mapcar
         (lambda (buffer)
           (with-current-buffer buffer
             (list
              gnus-group-line-format
              gnus-summary-line-format
              gnus-sum-thread-tree-root)))
         (list first second)))
    (mapc
     (lambda (buffer)
       (when (buffer-live-p buffer)
         (kill-buffer buffer)))
     (list first second))))
"##;
    let expect = expect![[
        r#"OK ((#("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display #6=(raise 0.0) font-lock-face #1=(:family #2="Material Icons" :height 1.2) face #1#)) #("%1{%U%R%z: %}%[%2{%&user-date;%}%]  %4{%-34,34n%} %3{ %}%(%1{%B%}%s%)\n" 35 36 (rear-nonsticky t display #7=(raise 0.0) font-lock-face #3=(:family #2# :height 1.2) face #3#) 54 55 (rear-nonsticky t display #8=(raise 0.0) font-lock-face #4=(:family #2# :height 1.2) face #4#)) #(" " 0 1 (rear-nonsticky t display #9=(raise 0.0) font-lock-face #5=(:family #2# :height 1.2) face #5#))) (#("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display #6# font-lock-face #1# face #1#)) #("%1{%U%R%z: %}%[%2{%&user-date;%}%]  %4{%-34,34n%} %3{ %}%(%1{%B%}%s%)\n" 35 36 (rear-nonsticky t display #7# font-lock-face #3# face #3#) 54 55 (rear-nonsticky t display #8# font-lock-face #4# face #4#)) #(" " 0 1 (rear-nonsticky t display #9# font-lock-face #5# face #5#))))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn thread_tree_contract_uses_icons_for_nodes_and_plain_space_for_vertical_edge() {
    let elisp_form = r##"
(all-the-icons-gnus--set-format)
(mapcar
 (lambda (symbol)
   (let ((value (symbol-value symbol)))
     (list
      symbol
      value
      (substring-no-properties value)
      (length value)
      (text-properties-at 0 value))))
 '(gnus-sum-thread-tree-root
   gnus-sum-thread-tree-false-root
   gnus-sum-thread-tree-single-indent
   gnus-sum-thread-tree-leaf-with-other
   gnus-sum-thread-tree-vertical
   gnus-sum-thread-tree-single-leaf))
"##;
    let expect = expect![[
        r#"OK ((gnus-sum-thread-tree-root #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #1=(:family #3="Material Icons" :height 1.2) face #1#)) " " 2 (rear-nonsticky t display (raise 0.0) font-lock-face #2=(:family "Material Icons" :height 1.2) face #2#)) (gnus-sum-thread-tree-false-root #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #4=(:family #3# :height 1.2) face #4#)) " " 2 (rear-nonsticky t display (raise 0.0) font-lock-face #5=(:family "Material Icons" :height 1.2) face #5#)) (gnus-sum-thread-tree-single-indent #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #6=(:family #3# :height 1.2) face #6#)) " " 2 (rear-nonsticky t display (raise 0.0) font-lock-face #7=(:family "Material Icons" :height 1.2) face #7#)) (gnus-sum-thread-tree-leaf-with-other #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #8=(:family #3# :height 1.2) face #8#)) " " 2 (rear-nonsticky t display (raise 0.0) font-lock-face #9=(:family "Material Icons" :height 1.2) face #9#)) (gnus-sum-thread-tree-vertical " " " " 1 nil) (gnus-sum-thread-tree-single-leaf #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #10=(:family #3# :height 1.2) face #10#)) " " 2 (rear-nonsticky t display (raise 0.0) font-lock-face #11=(:family "Material Icons" :height 1.2) face #11#)))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

use expect_test::expect;

use super::{assert_all_the_icons_gnus_autoload_parity, assert_all_the_icons_gnus_parity};

#[test]
fn public_setup_returns_last_format_value_and_matches_internal_formatter_state() {
    let elisp_form = r##"
(let ((result (all-the-icons-gnus-setup)))
  (list
   result
   (equal result gnus-sum-thread-tree-single-leaf)
   gnus-topic-line-format
   gnus-group-line-format
   gnus-summary-line-format
   gnus-user-date-format-alist
   gnus-sum-thread-tree-single-leaf))
"##;
    let expect = expect![[
        r#"OK (#(" " 0 1 (rear-nonsticky t display #8=(raise 0.0) font-lock-face #1=(:family #2="Material Icons" :height 1.2) face #1#)) t #("%i[  %(%{%n -- %A%}%) ]%v\n" 4 5 (rear-nonsticky t display (raise 0.0) font-lock-face #3=(:family #2# :height 1.2) face #3#)) #("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display (raise 0.0) font-lock-face #4=(:family #2# :height 1.2) face #4#)) #("%1{%U%R%z: %}%[%2{%&user-date;%}%]  %4{%-34,34n%} %3{ %}%(%1{%B%}%s%)\n" 35 36 (rear-nonsticky t display (raise 0.0) font-lock-face #5=(:family #2# :height 1.2) face #5#) 54 55 (rear-nonsticky t display (raise 0.0) font-lock-face #6=(:family #2# :height 1.2) face #6#)) ((t . #(" %Y-%m-%d %H:%M" 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #7=(:family #2# :height 1.2) face #7#)))) #(" " 0 1 (rear-nonsticky t display #8# font-lock-face #1# face #1#)))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn setup_does_not_compose_current_article_or_install_disabled_summary_advice() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "From:  : Alice\n")
  (let ((before (buffer-string))
        (advice-before
         (advice-member-p
          #'all-the-icons-gnus--add-faces
          #'gnus-summary-next-article)))
    (all-the-icons-gnus-setup)
    (list
     before
     (buffer-string)
     (text-properties-at (point-min))
     advice-before
     (advice-member-p
      #'all-the-icons-gnus--add-faces
      #'gnus-summary-next-article))))
"##;
    let expect = expect![[r#"OK ("From:  : Alice\n" "From:  : Alice\n" nil nil nil)"#]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn repeated_public_setup_is_idempotent_and_preserves_article_mapping_inventory() {
    let elisp_form = r##"
(let ((mapping-before (copy-tree pretty-gnus-article-alist)))
  (let ((first (all-the-icons-gnus-setup)))
    (let ((formats-first
           (list
            gnus-topic-line-format
            gnus-group-line-format
            gnus-summary-line-format
            gnus-user-date-format-alist
            gnus-sum-thread-tree-single-leaf)))
      (let ((second (all-the-icons-gnus-setup)))
        (list
         (equal first second)
         (equal
          formats-first
          (list
           gnus-topic-line-format
           gnus-group-line-format
           gnus-summary-line-format
           gnus-user-date-format-alist
           gnus-sum-thread-tree-single-leaf))
         (equal mapping-before pretty-gnus-article-alist)
         (length pretty-gnus-article-alist))))))
"##;
    let expect = expect!["OK (t t t 11)"];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

#[test]
fn autoloaded_setup_loads_source_dependencies_and_configures_gnus_in_one_call() {
    let elisp_form = r##"
(let ((before
       (list
        (featurep 'all-the-icons-gnus)
        (featurep 'all-the-icons)
        (featurep 'gnus))))
  (let ((result (all-the-icons-gnus-setup)))
    (list
     before
     (featurep 'all-the-icons-gnus)
     (featurep 'all-the-icons)
     (featurep 'gnus)
     (length pretty-gnus-article-alist)
     result
     gnus-group-line-format)))
"##;
    let expect = expect![[
        r#"OK ((nil nil nil) t t t 11 #(" " 0 1 (rear-nonsticky t display (raise 0.0) font-lock-face #1=(:family #2="Material Icons" :height 1.2) face #1#)) #("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display (raise 0.0) font-lock-face #3=(:family #2# :height 1.2) face #3#)))"#
    ]];
    assert_all_the_icons_gnus_autoload_parity(elisp_form, expect);
}

#[test]
fn setup_changes_only_documented_format_variables_and_preserves_unrelated_gnus_policy() {
    let elisp_form = r##"
(let ((gnus-use-cache 'passive)
      (gnus-use-agent nil)
      (gnus-check-new-newsgroups 'ask-server)
      (gnus-read-active-file 'some))
  (let ((before
         (list
          gnus-use-cache
          gnus-use-agent
          gnus-check-new-newsgroups
          gnus-read-active-file)))
    (all-the-icons-gnus-setup)
    (list
     before
     (list
      gnus-use-cache
      gnus-use-agent
      gnus-check-new-newsgroups
      gnus-read-active-file)
     gnus-topic-line-format
     gnus-group-line-format)))
"##;
    let expect = expect![[
        r#"OK ((passive nil ask-server some) (passive nil ask-server some) #("%i[  %(%{%n -- %A%}%) ]%v\n" 4 5 (rear-nonsticky t display (raise 0.0) font-lock-face #1=(:family #2="Material Icons" :height 1.2) face #1#)) #("%1M%1S%5y  : %(%-50,50G%)\n" 10 11 (rear-nonsticky t display (raise 0.0) font-lock-face #3=(:family #2# :height 1.2) face #3#)))"#
    ]];
    assert_all_the_icons_gnus_parity(elisp_form, expect);
}

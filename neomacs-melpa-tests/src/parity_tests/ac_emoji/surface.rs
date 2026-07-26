use expect_test::expect;

use super::assert_ac_emoji_parity;

#[test]
fn ac_emoji_exact_pin_dependencies_features_group_and_source_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-emoji package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-emoji ac-emoji-data auto-complete cl-lib))
                (get 'ac-emoji 'group-documentation)
                (get 'ac-emoji 'custom-group)
                ac-source-emoji
                (length ac-emoji--data)
                (length ac-emoji--candidates)))"##;
    let expect = expect![[
        r#"OK (ac-emoji "20150823.711" ((auto-complete (1 5 0)) (cl-lib (0 5))) (t t t t) "auto-complete source of Emoji." nil ((candidates . ac-emoji--candidates) (prefix . ":\\S-+")) 845 845)"#
    ]];

    assert_ac_emoji_parity(elisp_form, expect);
}

#[test]
fn ac_emoji_source_prefix_matches_complete_non_space_colon_tokens_only() {
    let elisp_form = r##"(let ((pattern
                    (cdr
                     (assq
                      'prefix
                      ac-source-emoji))))
               (mapcar
                (lambda (fixture)
                  (with-temp-buffer
                    (insert fixture)
                    (goto-char
                     (point-max))
                    (list
                     fixture
                     (and
                      (re-search-backward
                       pattern nil t)
                      (list
                       (match-string 0)
                       (match-beginning 0)
                       (match-end 0))))))
                '(":smile"
                  "prefix :heart_eyes"
                  ":"
                  "space :two words"
                  "newline\n:wave"
                  "plain"
                  ":a:b")))"##;
    let expect = expect![[
        r#"OK ((":smile" (":smile" 1 7)) ("prefix :heart_eyes" (":heart_eyes" 8 19)) (":" nil) ("space :two words" (":two" 7 11)) ("newline\n:wave" (":wave" 9 14)) ("plain" nil) (":a:b" (":b" 3 5)))"#
    ]];

    assert_ac_emoji_parity(elisp_form, expect);
}

#[test]
fn ac_emoji_candidates_are_popup_items_with_exact_data_derived_properties() {
    let elisp_form = r##"(mapcar
               (lambda (index)
                 (let ((data
                        (nth
                         index
                         ac-emoji--data))
                       (candidate
                        (nth
                         index
                         ac-emoji--candidates)))
                   (list
                    index
                    (plist-get data :key)
                    (substring-no-properties
                     candidate)
                    (text-properties-at
                     0 candidate)
                    (popup-item-documentation
                     candidate)
                    (popup-item-summary
                     candidate))))
               (list
                0
                1
                100
                500
                (1-
                 (length
                  ac-emoji--data))))"##;
    let expect = expect![[
        r#"OK ((0 ":smile:" ":smile:" (document "smiling face with open mouth and smiling eyes" summary "😄") "smiling face with open mouth and smiling eyes" "😄") (1 ":smiley:" ":smiley:" (document "smiling face with open mouth" summary "😃") "smiling face with open mouth" "😃") (100 ":ear:" ":ear:" (document "ear" summary "👂") "ear" "👂") (500 ":ramen:" ":ramen:" (document "steaming bowl" summary "🍜") "steaming bowl" "🍜") (844 ":small_blue_diamond:" ":small_blue_diamond:" (document "small blue diamond" summary "🔹") "small blue diamond" "🔹"))"#
    ]];

    assert_ac_emoji_parity(elisp_form, expect);
}

#[test]
fn ac_emoji_candidate_build_is_an_order_preserving_snapshot_of_data() {
    let elisp_form = r##"(let ((pairs
                    (cl-mapcar
                     (lambda
                         (data candidate)
                       (list
                        (equal
                         (plist-get data :key)
                         (substring-no-properties
                          candidate))
                        (equal
                         (plist-get
                          data :description)
                         (popup-item-documentation
                          candidate))
                        (equal
                         (plist-get
                          data :codepoint)
                         (popup-item-summary
                          candidate))))
                     ac-emoji--data
                     ac-emoji--candidates)))
               (list
                (length pairs)
                (cl-count
                 '(t t t)
                 pairs
                 :test #'equal)
                (cl-find-if
                 (lambda (pair)
                   (not
                    (equal pair
                           '(t t t))))
                 pairs)))"##;
    let expect = expect!["OK (845 845 nil)"];

    assert_ac_emoji_parity(elisp_form, expect);
}

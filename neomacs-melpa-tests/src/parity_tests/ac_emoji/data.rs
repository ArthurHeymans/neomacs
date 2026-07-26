use expect_test::expect;

use super::assert_ac_emoji_parity;

#[test]
fn ac_emoji_every_data_row_has_the_exact_key_codepoint_description_schema() {
    let elisp_form = r##"(let ((invalid
                    (cl-remove-if
                     (lambda (row)
                       (and
                        (listp row)
                        (= (length row) 6)
                        (equal
                         (cl-loop
                          for (key value) on row by #'cddr
                          collect key)
                         '(:key :codepoint :description))
                        (string-match-p
                         "\\`:[^[:space:]:]+:\\'"
                         (plist-get row :key))
                        (stringp
                         (plist-get
                          row :codepoint))
                        (= (length
                            (plist-get
                             row :codepoint))
                           1)
                        (stringp
                         (plist-get
                          row :description))
                        (not
                         (string-empty-p
                          (plist-get
                           row :description)))))
                     ac-emoji--data)))
               (list
                (length ac-emoji--data)
                (length invalid)
                invalid))"##;
    let expect = expect!["OK (845 0 nil)"];

    assert_ac_emoji_parity(elisp_form, expect);
}

#[test]
fn ac_emoji_keys_are_unique_and_cover_ascii_punctuation_digits_and_words() {
    let elisp_form = r##"(let* ((keys
                     (mapcar
                      (lambda (row)
                        (plist-get row :key))
                      ac-emoji--data))
                    (unique
                     (delete-dups
                      (copy-sequence keys))))
               (list
                (length keys)
                (length unique)
                (mapcar
                 (lambda (key)
                   (list
                    key
                    (cl-count
                     key keys
                     :test #'equal)))
                 '(":smile:"
                   ":+1:"
                   ":-1:"
                   ":100:"
                   ":e-mail:"
                   ":u7121:"
                   ":clock1230:"))))"##;
    let expect = expect![[
        r#"OK (845 845 ((":smile:" 1) (":+1:" 1) (":-1:" 1) (":100:" 1) (":e-mail:" 1) (":u7121:" 1) (":clock1230:" 1)))"#
    ]];

    assert_ac_emoji_parity(elisp_form, expect);
}

#[test]
fn ac_emoji_representative_rows_preserve_bmp_non_bmp_ascii_and_flag_values() {
    let elisp_form = r##"(mapcar
               (lambda (key)
                 (let ((row
                        (cl-find
                         key
                         ac-emoji--data
                         :key
                         (lambda (item)
                           (plist-get
                            item :key))
                         :test #'equal)))
                   (list
                    key
                    (plist-get
                     row :codepoint)
                    (and row
                         (string-to-char
                          (plist-get
                           row :codepoint)))
                    (plist-get
                     row :description))))
               '(":smile:"
                 ":relaxed:"
                 ":one:"
                 ":hash:"
                 ":jp:"
                 ":copyright:"
                 ":clock1130:"))"##;
    let expect = expect![[
        r##"OK ((":smile:" "😄" 128516 "smiling face with open mouth and smiling eyes") (":relaxed:" "☺" 9786 "white smiling face") (":one:" "1" 49 "digit one + combining enclosing keycap") (":hash:" "#" 35 "number sign + combining enclosing keycap") (":jp:" "🇯" 127471 "regional indicator symbol letter j + regional indicator symbol letter p") (":copyright:" "��" 169 "copyright sign") (":clock1130:" "🕦" 128358 "clock face eleven-thirty"))"##
    ]];

    assert_ac_emoji_parity(elisp_form, expect);
}

#[test]
fn ac_emoji_codepoint_alias_distribution_and_range_boundaries_match() {
    let elisp_form = r##"(let ((counts
                    (make-hash-table
                     :test 'equal))
                   minimum
                   maximum)
               (dolist (row ac-emoji--data)
                 (let* ((codepoint
                         (plist-get
                          row :codepoint))
                        (character
                         (string-to-char
                          codepoint)))
                   (puthash
                    codepoint
                    (1+
                     (gethash
                      codepoint counts 0))
                    counts)
                   (setq minimum
                         (if minimum
                             (min minimum
                                  character)
                           character)
                         maximum
                         (if maximum
                             (max maximum
                                  character)
                           character))))
               (let (duplicates)
                 (maphash
                  (lambda
                      (codepoint count)
                    (when (> count 1)
                      (push
                       (list
                        (string-to-char
                         codepoint)
                        count)
                       duplicates)))
                  counts)
                 (list
                  minimum
                  maximum
                  (hash-table-count counts)
                  (length duplicates)
                  (sort
                   duplicates
                   (lambda (left right)
                     (< (car left)
                        (car right)))))))"##;
    let expect = expect!["OK (35 128709 845 0 nil)"];

    assert_ac_emoji_parity(elisp_form, expect);
}

#[test]
fn ac_emoji_data_and_candidates_remain_independent_after_load() {
    let elisp_form = r##"(let ((original-data-length
                    (length ac-emoji--data))
                   (original-candidate-length
                    (length
                     ac-emoji--candidates))
                   (row
                    '(:key
                      ":neomacs_fixture:"
                      :codepoint "λ"
                      :description
                      "fixture")))
               (setq ac-emoji--data
                     (cons
                      row ac-emoji--data))
               (list
                original-data-length
                original-candidate-length
                (length ac-emoji--data)
                (length
                 ac-emoji--candidates)
                (cl-find
                 ":neomacs_fixture:"
                 ac-emoji--candidates
                 :test #'equal)
                (car ac-emoji--data)
                (substring-no-properties
                 (car
                  ac-emoji--candidates))))"##;
    let expect = expect![[
        r#"OK (845 845 846 845 nil (:key ":neomacs_fixture:" :codepoint "λ" :description "fixture") ":smile:")"#
    ]];

    assert_ac_emoji_parity(elisp_form, expect);
}

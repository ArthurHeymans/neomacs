use expect_test::expect;

use super::assert_ample_regexps_parity;

#[test]
fn structured_log_regexp_extracts_timestamp_level_component_and_message() {
    let elisp_form = r##"(progn
  (define-arx log-rx
    '((digits (+ digit))
      (timestamp
       (seq
        (group-n 1 (= 4 digit)) "-"
        (group-n 2 (= 2 digit)) "-"
        (group-n 3 (= 2 digit)) "T"
        (group-n 4 (= 2 digit)) ":"
        (group-n 5 (= 2 digit)) ":"
        (group-n 6 (= 2 digit)) "Z"))
      (level
       (group-n 7
                (or "TRACE" "DEBUG" "INFO"
                    "WARN" "ERROR")))
      (component
       (group-n 8
                (regexp "[[:alnum:]_.-]+")))
      (message (group-n 9 (+ nonl)))
      (record
       (seq line-start timestamp blank
            "[" level "]" blank
            component ":" blank message line-end))))
  (let* ((regexp (log-rx record))
         (line
          "2026-07-27T10:42:09Z [WARN] cache.worker: stale entry 42")
         (matched (string-match regexp line)))
    (list
     regexp
     matched
     (and matched
          (mapcar
           (lambda (index) (match-string index line))
           '(1 2 3 4 5 6 7 8 9)))
     (string-match-p
      regexp
      "2026-07-27 10:42:09 [WARN] cache.worker: stale entry"))))"##;
    let expect = expect![[
        r#"OK ("^\\(?1:[[:digit:]]\\{4\\}\\)-\\(?2:[[:digit:]]\\{2\\}\\)-\\(?3:[[:digit:]]\\{2\\}\\)T\\(?4:[[:digit:]]\\{2\\}\\):\\(?5:[[:digit:]]\\{2\\}\\):\\(?6:[[:digit:]]\\{2\\}\\)Z[[:blank:]]\\[\\(?7:\\(?:DEBUG\\|ERROR\\|INFO\\|TRACE\\|WARN\\)\\)][[:blank:]]\\(?8:[[:alnum:]_.-]+\\):[[:blank:]]\\(?9:.+\\)$" 0 ("2026" "07" "27" "10" "42" "09" "WARN" "cache.worker" "stale entry 42") nil)"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn generated_regexp_searches_a_real_buffer_and_returns_every_assignment() {
    let elisp_form = r##"(progn
  (define-arx assignment-rx
    '((identifier
       (regexp "[[:alpha:]_][[:alnum:]_-]*"))
      (horizontal-space (* (any " \t")))
      (assignment
       (seq line-start horizontal-space
            (group identifier)
            horizontal-space "=" horizontal-space
            (group (* nonl))
            line-end))))
  (with-temp-buffer
    (insert
     "# release configuration\n"
     "channel = stable\n"
     "workers=24\n"
     "invalid line\n"
     "artifact_name = neomacs-linux-x86_64\n")
    (goto-char (point-min))
    (let ((regexp (assignment-rx assignment))
          matches)
      (while (re-search-forward regexp nil t)
        (push
         (list
          (match-string-no-properties 1)
          (match-string-no-properties 2)
          (line-number-at-pos (match-beginning 0)))
         matches))
      (nreverse matches))))"##;
    let expect = expect![[
        r#"OK (("channel" "stable" 2) ("workers" "24" 3) ("artifact_name" "neomacs-linux-x86_64" 5))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn generated_regexp_performs_practical_semantic_version_replacement() {
    let elisp_form = r##"(progn
  (define-arx version-rx
    '((number (+ digit))
      (version
       (seq
        word-start
        "v"
        (group-n 1 number) "."
        (group-n 2 number) "."
        (group-n 3 number)
        word-end))))
  (let ((regexp (version-rx version))
        (text
         "upgrade v1.2.3, keep v20.0.17, ignore version-3 and v1.2"))
    (list
     regexp
     (replace-regexp-in-string
      regexp
      (lambda (matched)
        (format
         "release-%s_%s_%s"
         (match-string 1 matched)
         (match-string 2 matched)
         (match-string 3 matched)))
      text)
     (split-string text regexp t))))"##;
    let expect = expect![[
        r#"OK ("\\<v\\(?1:[[:digit:]]+\\)\\.\\(?2:[[:digit:]]+\\)\\.\\(?3:[[:digit:]]+\\)\\>" "upgrade release-1_2_3, keep release-20_0_17, ignore version-3 and v1.2" ("upgrade " ", keep " ", ignore version-3 and v1.2"))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn unicode_identifier_regexp_filters_multilingual_tokens_and_rejects_punctuation() {
    let elisp_form = r##"(progn
  (define-arx identifier-rx
    '((identifier
       (seq symbol-start
            (group
             (regexp "[[:alpha:]_][[:alnum:]_]*"))
            symbol-end))))
  (let ((regexp (identifier-rx identifier)))
    (mapcar
     (lambda (text)
       (let ((matched (string-match regexp text)))
         (list
          text
          (and matched t)
          (and matched (match-string 1 text))
          (and matched (match-beginning 0))
          (and matched (match-end 0)))))
     '("naïve_value"
       "变量42"
       "_private"
       "42invalid"
       "alpha-beta"
       "λ-calculus"))))"##;
    let expect = expect![[
        r#"OK (("naïve_value" t "naïve_value" 0 11) ("变量42" t "变量42" 0 4) ("_private" t "_private" 0 8) ("42invalid" nil nil nil nil) ("alpha-beta" nil nil nil nil) ("λ-calculus" nil nil nil nil))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn nested_aliases_preserve_alternation_precedence_inside_repetition_and_suffixes() {
    let elisp_form = r##"(progn
  (define-arx endpoint-rx
    '((scheme (or "http" "https"))
      (host-part
       (regexp "[[:alnum:]][[:alnum:]-]*"))
      (host
       (seq host-part
            (* "." host-part)))
      (port
       (seq ":" (+ digit)))
      (endpoint
       (seq line-start scheme "://" host
            (opt port)
            (opt "/")
            line-end))))
  (let ((regexp (endpoint-rx endpoint)))
    (list
     regexp
     (mapcar
      (lambda (text)
        (list text (and (string-match-p regexp text) t)))
      '("https://example.com"
        "http://api-2.example.com:8443/"
        "ftp://example.com"
        "https://-bad.example"
        "https://example.com/path"
        "prefix https://example.com")))))"##;
    let expect = expect![[
        r#"OK ("^\\(?:https?\\)://\\(?:[[:alnum:]][[:alnum:]-]*\\)\\(?:\\.\\(?:[[:alnum:]][[:alnum:]-]*\\)\\)*\\(?::[[:digit:]]+\\)?/?$" (("https://example.com" t) ("http://api-2.example.com:8443/" t) ("ftp://example.com" nil) ("https://-bad.example" nil) ("https://example.com/path" nil) ("prefix https://example.com" nil)))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn runtime_generated_regexp_uses_dynamic_form_data_for_real_field_validation() {
    let elisp_form = r##"(progn
  (define-arx field-rx
    '((field
       (:func
        (lambda (_form names)
          `(or ,@names))
        :min-args 1
        :max-args 1))
      (value (regexp "[^=\n]+"))
      (record
       (seq line-start
            (group (field ("name" "email" "team")))
            "="
            (group value)
            line-end))))
  (let ((regexp (field-rx-to-string 'record t)))
    (mapcar
     (lambda (line)
       (let ((matched (string-match regexp line)))
         (list
          line
          (and matched t)
          (and matched (match-string 1 line))
          (and matched (match-string 2 line)))))
     '("name=Ada"
       "email=ada@example.test"
       "team=runtime"
       "role=maintainer"
       "name="))))"##;
    let expect = expect![[
        r#"OK (("name=Ada" t "name" "Ada") ("email=ada@example.test" t "email" "ada@example.test") ("team=runtime" t "team" "runtime") ("role=maintainer" nil nil nil) ("name=" nil nil nil))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn custom_raw_regexp_form_preserves_power_user_grouping_and_quantifier_behavior() {
    let elisp_form = r##"(progn
  (define-arx raw-rx
    '((raw-word
       (:func
        (lambda (_form) "foo\\|bar")))
      (safe-word
       (:func
        (lambda (_form)
          '(or "foo" "bar"))))))
  (let ((raw-star (raw-rx (* (raw-word))))
        (safe-star (raw-rx (* (safe-word)))))
    (list
     raw-star safe-star
     (mapcar
      (lambda (text)
        (list
         text
         (and (string-match-p
               (concat "\\`" raw-star "\\'") text)
              t)
         (and (string-match-p
               (concat "\\`" safe-star "\\'") text)
              t)))
      '("" "foo" "bar" "foobar" "barbar" "fo" "baar")))))"##;
    let expect = expect![[
        r#"OK ("\\(?:foo\\|bar\\)*" "\\(?:bar\\|foo\\)*" (("" t t) ("foo" t t) ("bar" t t) ("foobar" t t) ("barbar" t t) ("fo" nil nil) ("baar" nil nil)))"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}

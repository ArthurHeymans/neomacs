use expect_test::expect;

use super::assert_ac_mozc_parity;

/// The package's headline story: a user writing Japanese types the romaji
/// `kanji' at the end of a Japanese sentence and auto-complete offers the
/// readings and the kanji conversions of that word.  This pins the candidate
/// list in order (both phases: the readings mozc offers for the preedit, then
/// the conversions it offers after the conversion key), the prefix and its
/// starting point, the complete key-by-key conversation ac-mozc had with the
/// helper -- one `SendKey' per romaji letter, then the space that asks for the
/// conversion -- and that starting completion left the buffer text alone.
#[test]
fn typing_romaji_offers_the_kana_reading_and_its_kanji_conversions() {
    let elisp_form = r##"(progn
  (ac-mozc-test-setup)
  (ac-mozc-test-with-buffer
   'ac-source-mozc "今日の天気はkanji"
   (let* ((candidates (ac-mozc-test-complete))
          (prefix ac-prefix)
          (point ac-point)
          (symbol (get-text-property 0 'symbol (car ac-candidates)))
          (action (get-text-property 0 'action (car ac-candidates))))
     (ac-abort)
     (list :candidates candidates
           :prefix prefix
           :prefix-start point
           :prefix-function (ac-mozc-prefix)
           :popup-symbol symbol
           :popup-action action
           :buffer (buffer-string)
           :point (point)
           :traffic (ac-mozc-test-traffic)))))"##;
    let expect = expect![[
        r#"OK (:candidates ("かんじ" "カンジ" "漢字" "感じ" "幹事") :prefix "kanji" :prefix-start 7 :prefix-function 7 :popup-symbol "M" :popup-action ac-mozc-action :buffer "今日の天気はkanji" :point 12 :traffic (("start" "--suppress_stderr") ("(0 CreateSession)") ("(1 SendKey 1 107)") ("(2 SendKey 1 97)") ("(3 SendKey 1 110)") ("(4 SendKey 1 106)") ("(5 SendKey 1 105)") ("(6 SendKey 1 space)")))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

/// Choosing a candidate replaces the romaji with the Japanese word, and
/// `ac-mozc-action' then removes the space that separated the romaji from the
/// preceding word -- the whole point of `ac-mozc-remove-space', because a
/// Japanese sentence has no space there.  With the option turned off the same
/// completion keeps the space, and both runs leave `ac-mozc-ac-point' cleared.
#[test]
fn completing_inserts_the_japanese_word_and_removes_the_space_before_it() {
    let elisp_form = r##"(progn
  (ac-mozc-test-setup)
  (list
   :remove-space
   (ac-mozc-test-with-buffer
    'ac-source-mozc "hello ohayou"
    (ac-mozc-test-complete)
    (ac-complete)
    (list (buffer-string) (point) ac-mozc-ac-point ac-mozc-remove-space))
   :keep-space
   (ac-mozc-test-with-buffer
    'ac-source-mozc "hello ohayou"
    (let ((ac-mozc-remove-space nil))
      (ac-mozc-test-complete)
      (ac-complete)
      (list (buffer-string) (point) ac-mozc-ac-point ac-mozc-remove-space)))))"##;
    let expect = expect![[
        r#"OK (:remove-space ("helloおはよう" 10 nil t) :keep-space ("hello おはよう" 11 nil nil))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

/// `ac-mozc-prefix' decides what counts as romaji waiting to be converted: a
/// run of letters and sentence punctuation, starting after anything that is
/// not one of those.  It has to start right after Japanese text (there is no
/// space to separate them), it must not reach back over a space, and digits
/// alone are not romaji at all.
#[test]
fn the_prefix_is_the_romaji_run_that_ends_at_point() {
    let elisp_form = r##"(progn
  (ac-mozc-test-setup)
  (ac-mozc-test-with-buffer
   'ac-source-mozc ""
   (let ((observed nil))
     (dolist (text '("日本語のnihongo" "kanji" "foo bar-baz" "123" "abc " "" "?!"))
       (erase-buffer)
       (insert text)
       (push (list text (ac-mozc-prefix)) observed))
     (erase-buffer)
     (insert "日本語のnihongo")
     (let* ((candidates (ac-mozc-test-complete))
            (prefix ac-prefix)
            (start ac-point))
       (ac-abort)
       (list :prefixes (nreverse observed)
             :candidates candidates
             :prefix prefix
             :prefix-start start)))))"##;
    let expect = expect![[
        r#"OK (:prefixes (("日本語のnihongo" 5) ("kanji" 1) ("foo bar-baz" 5) ("123" nil) ("abc " nil) ("" nil) ("?!" 1)) :candidates ("にほんご" "ニホンゴ" "日本語") :prefix "nihongo" :prefix-start 5)"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

/// The `ac-mozc-kana-p' guard: romaji that mozc cannot read as kana comes back
/// as full-width latin in the preedit, and ac-mozc then offers nothing rather
/// than the letters themselves.  The traffic proves the difference is decided
/// after the keys were sent and before the conversion key -- the space that
/// asks mozc to convert is never sent for this word.
#[test]
fn input_that_mozc_cannot_read_as_kana_offers_nothing_and_is_never_converted() {
    let elisp_form = r##"(progn
  (ac-mozc-test-setup)
  (list
   :not-kana
   (ac-mozc-test-with-buffer
    'ac-source-mozc "xyz"
    (let ((candidates (ac-mozc-test-complete)))
      (ac-abort)
      (list candidates (ac-mozc-test-traffic))))
   :kana
   (ac-mozc-test-with-buffer
    'ac-source-mozc "kanji"
    (let ((candidates (ac-mozc-test-complete)))
      (ac-abort)
      (list candidates (last (ac-mozc-test-traffic) 2))))))"##;
    let expect = expect![[
        r#"OK (:not-kana (nil (("start" "--suppress_stderr") ("(0 CreateSession)") ("(1 SendKey 1 120)") ("(2 SendKey 1 121)") ("(3 SendKey 1 122)"))) :kana (("かんじ" "カンジ" "漢字" "感じ" "幹事") (("(9 SendKey 2 105)") ("(10 SendKey 2 space)"))))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

/// The package's second source completes the ASCII words already written in
/// the surrounding Japanese text -- identifiers, product names -- without any
/// mozc session.  A word glued to Japanese characters (`変数名myVariable') is
/// still offered as its ASCII run alone, matching happens at word starts, and
/// only buffers in the same major mode contribute.
#[test]
fn the_ascii_word_source_completes_words_embedded_in_japanese_text() {
    let elisp_form = r##"(progn
  (ac-mozc-test-setup)
  (let ((notes (generate-new-buffer "*notes*"))
        (program (generate-new-buffer "*program*")))
    (unwind-protect
        (progn
          (with-current-buffer notes
            (text-mode)
            (insert "変数名myVariable を使う\nmyFunction(引数)\n日本語only\nplain english words\n"))
          (with-current-buffer program
            (prog-mode)
            (insert "myProgModeWord\n"))
          (ac-mozc-test-with-buffer
           'ac-source-ascii-words-in-same-mode-buffers "私はmy"
           (let ((candidates (ac-mozc-test-complete))
                 (prefix ac-prefix)
                 (start ac-point))
             (ac-abort)
             (erase-buffer)
             (insert "日本語のplain")
             (let ((second (ac-mozc-test-complete)))
               (ac-abort)
               (list :candidates candidates
                     :prefix prefix
                     :prefix-start start
                     :second second
                     :split (ac-mozc-remove-non-ascii-character
                             '("変数名myVariable" "日本語only" "plain" "全角のみ"))
                     :partial (ac-mozc-partial-match
                               "my" '("myVariable" "myFunction" "dummyValue" "plain"))
                     :traffic (ac-mozc-test-traffic))))))
      (kill-buffer notes)
      (kill-buffer program))))"##;
    let expect = expect![[
        r#"OK (:candidates ("myVariable" "myFunction") :prefix "my" :prefix-start 3 :second ("plain") :split ("myVariable" "only" "plain") :partial ("myVariable" "myFunction") :traffic nothing-recorded)"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

/// The two ways the helper can let the user down.  When `mozc_emacs_helper' is
/// not installed at all, starting completion signals
/// `mozc-helper-process-error' and mozc.el reports it; when the helper greets
/// and then dies mid-session, ac-mozc's own "Mozc session failed." error is
/// raised instead.  Both leave `mozc-mode' disabled by `mozc-abort', nothing
/// completed, and -- because the error escaped before `ac-cleanup' ran -- the
/// blank line auto-complete had reserved for its menu still in the buffer.
#[test]
fn a_missing_or_dying_mozc_helper_reports_the_failure_and_disables_mozc_mode() {
    let elisp_form = r##"(progn
  (ac-mozc-test-setup)
  (list
   :missing
   (let ((mozc-helper-program-name "mozc_emacs_helper_not_installed")
         (mark (ac-mozc-test-message-mark)))
     (setq mozc-helper-process nil mozc-session-id nil)
     (ac-mozc-test-with-buffer
      'ac-source-mozc "kanji"
      (list (condition-case error (ac-mozc-test-complete) (error error))
            mozc-mode
            (buffer-string)
            (ac-mozc-test-messages-since mark))))
   :dying
   (let ((mozc-helper-program-name
          (expand-file-name "mozc_emacs_helper_dying" ac-mozc-test-bin))
         (mark (ac-mozc-test-message-mark)))
     (setq mozc-helper-process nil mozc-session-id nil)
     (ac-mozc-test-with-buffer
      'ac-source-mozc "kanji"
      (list (condition-case error (ac-mozc-test-complete) (error error))
            mozc-mode
            (buffer-string)
            (ac-mozc-test-messages-since mark))))))"##;
    let expect = expect![[
        r#"OK (:missing ((mozc-helper-process-error) nil "kanji\n" ("mozc.el: Starting mozc-helper-process..." "mozc.el: Failed to start mozc-helper-process.")) :dying ((error "Mozc session failed.") nil "kanji\n" ("mozc.el: Starting mozc-helper-process...done" "mozc.el: No response from the server")))"#
    ]];

    assert_ac_mozc_parity(elisp_form, expect);
}

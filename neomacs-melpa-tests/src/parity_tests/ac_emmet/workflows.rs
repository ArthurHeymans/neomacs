use expect_test::expect;

use super::{assert_ac_emmet_parity, assert_unshimmed_ac_emmet_parity};

/// The package's primary story: set up the html source in a real `html-mode'
/// buffer, type an abbreviation, and let `ac-complete' run the source's action
/// so emmet expands it into markup.
#[test]
fn completing_an_html_abbreviation_expands_it_into_real_markup() {
    let elisp_form = r####"
(ac-emmet-test-in-buffer
 #'html-mode "*ac-emmet-html-workflow*"
 (ac-emmet-html-setup)
 (insert "btn")
 (let* ((candidates (ac-emmet-test-candidates))
        (prefix ac-prefix)
        (metadata (ac-emmet-test-items))
        first
        second)
   (ac-complete)
   (setq first (list :buffer (buffer-string) :point (point)))
   (goto-char (point-max))
   (insert "\nart")
   (let ((next (ac-emmet-test-candidates))
         (next-prefix ac-prefix))
     (ac-complete)
     (setq second (list :candidates next
                        :prefix next-prefix
                        :buffer (buffer-string)
                        :point (point))))
   (list :mode major-mode
         :emmet-mode emmet-mode
         :css-transform emmet-use-css-transform
         :sources ac-sources
         :global-sources (default-value 'ac-sources)
         :prefix prefix
         :candidates candidates
         :metadata metadata
         :first first
         :second second
         :modified (buffer-modified-p))))
"####;
    let expect = expect![[
        r#"OK (:mode html-mode :emmet-mode t :css-transform nil :sources (ac-source-emmet-html-aliases ac-source-emmet-html-snippets) :global-sources (ac-source-words-in-same-mode-buffers) :prefix "btn" :candidates ("btn" "btn:b" "btn:r" "btn:s") :metadata ((:candidate "btn" :documentation "button" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1=(lambda nil (call-interactively 'emmet-expand-line))) (:candidate "btn:b" :documentation "button[type=button]" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#) (:candidate "btn:r" :documentation "button[type=reset]" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#) (:candidate "btn:s" :documentation "button[type=submit]" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#)) :first (:buffer "<button></button>" :point 9) :second (:candidates ("art") :prefix "art" :buffer "<button></button>\n<article></article>" :point 28) :modified t)"#
    ]];

    assert_ac_emmet_parity(elisp_form, expect);
}

/// The same story on the css side.  `emmet-mode' is what sets
/// `emmet-use-css-transform' for `css-mode', which is why the identical action
/// produces a CSS declaration here and markup in the html workflow.
#[test]
fn completing_a_css_abbreviation_expands_it_into_a_real_declaration() {
    let elisp_form = r####"
(ac-emmet-test-in-buffer
 #'css-mode "*ac-emmet-css-workflow*"
 (ac-emmet-css-setup)
 (insert "pos")
 (let* ((candidates (ac-emmet-test-candidates))
        (prefix ac-prefix)
        (metadata (ac-emmet-test-items))
        first
        second)
   (ac-complete)
   (setq first (list :buffer (buffer-string) :point (point)))
   (goto-char (point-max))
   (insert "\nai")
   (let ((next (ac-emmet-test-candidates))
         (next-prefix ac-prefix))
     (ac-complete)
     (setq second (list :candidates next
                        :prefix next-prefix
                        :buffer (buffer-string)
                        :point (point))))
   (list :mode major-mode
         :emmet-mode emmet-mode
         :css-transform emmet-use-css-transform
         :sources ac-sources
         :global-sources (default-value 'ac-sources)
         :prefix prefix
         :candidates candidates
         :metadata metadata
         :first first
         :second second
         :modified (buffer-modified-p))))
"####;
    let expect = expect![[
        r#"OK (:mode css-mode :emmet-mode t :css-transform t :sources (ac-source-emmet-css-snippets) :global-sources (ac-source-words-in-same-mode-buffers) :prefix "pos" :candidates ("pos" "pos:a" "pos:f" "pos:r" "pos:s") :metadata ((:candidate "pos" :documentation "position:${1:relative};" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1=(lambda nil (call-interactively 'emmet-expand-line))) (:candidate "pos:a" :documentation "position:absolute;" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#) (:candidate "pos:f" :documentation "position:fixed;" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#) (:candidate "pos:r" :documentation "position:relative;" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#) (:candidate "pos:s" :documentation "position:static;" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#)) :first (:buffer "position: relative;" :point 20) :second (:candidates ("ai" "ai:b" "ai:c" "ai:s" "ai:fe" "ai:fs") :prefix "ai" :buffer "position: relative;\nalign-items: ;" :point 34) :modified t)"#
    ]];

    assert_ac_emmet_parity(elisp_form, expect);
}

/// What the user reads in the popup: the documentation auto-complete shows for
/// a candidate is the emmet snippet the abbreviation will expand to, resolved
/// through the source's `document' function, and each candidate carries the
/// package's own faces and symbol.
#[test]
fn candidate_documentation_and_popup_metadata_come_from_emmets_snippet_tables() {
    let elisp_form = r####"
(list
 :snapshot-sizes
 (list (length ac-emmet-html-snippets-keys)
       (length ac-emmet-html-aliases-keys)
       (length ac-emmet-css-snippets-keys))
 :html-snippet-keys
 (sort (copy-sequence ac-emmet-html-snippets-keys) #'string<)
 :html
 (ac-emmet-test-in-buffer
  #'html-mode "*ac-emmet-documentation-html*"
  (ac-emmet-html-setup)
  (list (ac-emmet-test-offer-with-metadata "cc")
        (ac-emmet-test-offer-with-metadata "bq")))
 :css
 (ac-emmet-test-in-buffer
  #'css-mode "*ac-emmet-documentation-css*"
  (ac-emmet-css-setup)
  (list (ac-emmet-test-offer-with-metadata "colmr")))
 :sources
 (list (cdr (assq 'candidates ac-source-emmet-html-snippets))
       (cdr (assq 'candidates ac-source-emmet-html-aliases))
       (cdr (assq 'candidates ac-source-emmet-css-snippets))
       (cdr (assq 'requires ac-emmet-source-defaults))
       (cdr (assq 'symbol ac-emmet-source-defaults))))
"####;
    let expect = expect![[
        r#"OK (:snapshot-sizes (9 112 641) :html-snippet-keys ("!!!" "!!!4s" "!!!4t" "!!!xs" "!!!xt" "!!!xxs" "cc:ie" "cc:ie6" "cc:noie") :html ((:typed "cc" :prefix "cc" :candidates ("cc:ie" "cc:ie6" "cc:noie") :metadata ((:candidate "cc:ie" :documentation "<!--[if IE]>\n\11${child}\n<![endif]-->" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1=(lambda nil (call-interactively 'emmet-expand-line))) (:candidate "cc:ie6" :documentation "<!--[if lte IE 6]>\n\11${child}\n<![endif]-->" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#) (:candidate "cc:noie" :documentation "<!--[if !IE]><!-->\n\11${child}\n<!--<![endif]-->" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#))) (:typed "bq" :prefix "bq" :candidates ("bq") :metadata ((:candidate "bq" :documentation "blockquote" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#)))) :css ((:typed "colmr" :prefix "colmr" :candidates ("colmr" "colmrc" "colmrs" "colmrw") :metadata ((:candidate "colmr" :documentation "column-rule:|;" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#) (:candidate "colmrc" :documentation "column-rule-color:|;" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#) (:candidate "colmrs" :documentation "column-rule-style:|;" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#) (:candidate "colmrw" :documentation "column-rule-width:|;" :summary nil :symbol "a" :candidate-face ac-emmet-candidate-face :selection-face ac-emmet-selection-face :expands-with #1#)))) :sources (ac-emmet-html-snippets-keys ac-emmet-html-aliases-keys ac-emmet-css-snippets-keys 1 "a"))"#
    ]];

    assert_ac_emmet_parity(elisp_form, expect);
}

/// Both setup commands are buffer-local, so an html buffer must never offer a
/// css abbreviation and a css buffer must never offer an html one, and neither
/// may touch the global `ac-sources'.
#[test]
fn the_html_source_is_not_offered_in_css_and_the_css_source_is_not_offered_in_html() {
    let elisp_form = r####"
(let ((html-buffer (generate-new-buffer "*ac-emmet-isolation-html*"))
      (css-buffer (generate-new-buffer "*ac-emmet-isolation-css*"))
      (global-before (default-value 'ac-sources))
      html
      css)
  (unwind-protect
      (progn
        (set-window-buffer (selected-window) html-buffer)
        (set-buffer html-buffer)
        (html-mode)
        (emmet-mode 1)
        (setq ac-sources nil)
        (auto-complete-mode 1)
        (ac-emmet-html-setup)
        (setq html
              (list :mode major-mode
                    :sources ac-sources
                    :html-abbreviation (ac-emmet-test-offer "bq")
                    :css-abbreviation (ac-emmet-test-offer "pos")))
        (set-window-buffer (selected-window) css-buffer)
        (set-buffer css-buffer)
        (css-mode)
        (emmet-mode 1)
        (setq ac-sources nil)
        (auto-complete-mode 1)
        (ac-emmet-css-setup)
        (setq css
              (list :mode major-mode
                    :sources ac-sources
                    :css-abbreviation (ac-emmet-test-offer "pos")
                    :html-abbreviation (ac-emmet-test-offer "bq"))))
    (kill-buffer html-buffer)
    (kill-buffer css-buffer))
  (list :html html
        :css css
        :global-sources-before global-before
        :global-sources-after (default-value 'ac-sources)
        :global-untouched (equal global-before (default-value 'ac-sources))
        :setup-commands (list (commandp 'ac-emmet-html-setup)
                              (commandp 'ac-emmet-css-setup))))
"####;
    let expect = expect![[
        r#"OK (:html (:mode html-mode :sources (ac-source-emmet-html-aliases ac-source-emmet-html-snippets) :html-abbreviation (:typed "bq" :prefix "bq" :candidates ("bq")) :css-abbreviation (:typed "pos" :prefix "pos" :candidates nil)) :css (:mode css-mode :sources (ac-source-emmet-css-snippets) :css-abbreviation (:typed "pos" :prefix "pos" :candidates ("pos" "pos:a" "pos:f" "pos:r" "pos:s")) :html-abbreviation (:typed "bq" :prefix "bq" :candidates nil)) :global-sources-before #1=(ac-source-words-in-same-mode-buffers) :global-sources-after #1# :global-untouched t :setup-commands (t t))"#
    ]];

    assert_ac_emmet_parity(elisp_form, expect);
}

/// `requires . 1' means one character is enough to start completing, and each
/// further character narrows the list.  The colon in an abbreviation behaves
/// differently in the two modes because auto-complete's default prefix follows
/// the buffer's syntax table.
#[test]
fn typing_more_characters_narrows_the_offered_abbreviations() {
    let elisp_form = r####"
(list
 :html
 (ac-emmet-test-in-buffer
  #'html-mode "*ac-emmet-narrowing-html*"
  (ac-emmet-html-setup)
  (mapcar #'ac-emmet-test-offer
          '("" "b" "bt" "btn" "btn:" "btn:b" "btn:bb")))
 :css
 (ac-emmet-test-in-buffer
  #'css-mode "*ac-emmet-narrowing-css*"
  (ac-emmet-css-setup)
  (mapcar #'ac-emmet-test-offer
          '("" "colm" "colmr" "colmrs" "colmrsx" "ai" "ai:" "ai:fs")))
 :requires (cdr (assq 'requires ac-emmet-source-defaults)))
"####;
    let expect = expect![[
        r#"OK (:html ((:typed "" :prefix nil :candidates nil) (:typed "b" :prefix "b" :candidates ("bq" "btn" "bdo:l" "bdo:r" "btn:b" "btn:r" "btn:s")) (:typed "bt" :prefix "bt" :candidates ("btn" "btn:b" "btn:r" "btn:s")) (:typed "btn" :prefix "btn" :candidates ("btn" "btn:b" "btn:r" "btn:s")) (:typed "btn:" :prefix "btn:" :candidates ("btn:b" "btn:r" "btn:s")) (:typed "btn:b" :prefix "btn:b" :candidates ("btn:b")) (:typed "btn:bb" :prefix "btn:bb" :candidates nil)) :css ((:typed "" :prefix nil :candidates nil) (:typed "colm" :prefix "colm" :candidates ("colm" "colmc" "colmf" "colmg" "colmr" "colms" "colmw" "colmrc" "colmrs" "colmrw")) (:typed "colmr" :prefix "colmr" :candidates ("colmr" "colmrc" "colmrs" "colmrw")) (:typed "colmrs" :prefix "colmrs" :candidates ("colmrs")) (:typed "colmrsx" :prefix "colmrsx" :candidates nil) (:typed "ai" :prefix "ai" :candidates ("ai" "ai:b" "ai:c" "ai:s" "ai:fe" "ai:fs")) (:typed "ai:" :prefix nil :candidates nil) (:typed "ai:fs" :prefix "fs" :candidates ("fs" "fsm" "fst" "fs:i" "fs:n" "fs:o" "fsm:a" "fsm:n" "fst:c" "fst:e" "fst:n" "fsm:aw" "fst:ec" "fst:ee" "fst:sc" "fst:se" "fst:uc" "fst:ue"))) :requires 1)"#
    ]];

    assert_ac_emmet_parity(elisp_form, expect);
}

/// A realistic miss: an abbreviation emmet has no snippet for offers nothing,
/// `ac-complete' must not expand or damage the surrounding markup, and the next
/// real abbreviation must still complete.
#[test]
fn an_abbreviation_with_no_snippet_leaves_the_buffer_untouched() {
    let elisp_form = r####"
(ac-emmet-test-in-buffer
 #'html-mode "*ac-emmet-no-match-workflow*"
 (ac-emmet-html-setup)
 (insert "<p>keep me</p>\nzzqq")
 (let* ((before (buffer-string))
        (point-before (point))
        (candidates (ac-emmet-test-candidates))
        (prefix ac-prefix))
   (ac-complete)
   (let ((after (buffer-string))
         (point-after (point)))
     (goto-char (point-max))
     (insert "\nbq")
     (let ((recovered (ac-emmet-test-candidates)))
       (ac-complete)
       (list :before before
             :point-before point-before
             :prefix prefix
             :candidates candidates
             :after after
             :point-after point-after
             :buffer-unchanged (equal before after)
             :recovered-candidates recovered
             :final (buffer-string)
             :final-point (point))))))
"####;
    let expect = expect![[
        r#"OK (:before "<p>keep me</p>\nzzqq" :point-before 20 :prefix "zzqq" :candidates nil :after "<p>keep me</p>\nzzqq" :point-after 20 :buffer-unchanged t :recovered-candidates ("bq") :final "<p>keep me</p>\nzzqq\n<blockquote></blockquote>" :final-point 33)"#
    ]];

    assert_ac_emmet_parity(elisp_form, expect);
}

/// What a user actually gets today: ac-emmet builds its candidate lists with
/// cl.el's `loop' while the file loads, and modern Emacs no longer defines it,
/// so the package cannot be loaded at all without a compatibility shim.
#[test]
fn installing_ac_emmet_unshimmed_fails_on_the_legacy_loop_macro() {
    let elisp_form = r####"
(list (featurep 'ac-emmet)
      (boundp 'ac-emmet-html-snippets-keys)
      (fboundp 'ac-emmet-html-setup))
"####;
    let expect = expect!["ERR (void-function loop)"];

    assert_unshimmed_ac_emmet_parity(elisp_form, expect);
}

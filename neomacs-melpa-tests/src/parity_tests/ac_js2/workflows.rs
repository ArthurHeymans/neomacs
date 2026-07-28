use expect_test::expect;

use super::assert_ac_js2_parity;

/// The documented installation, `(add-hook 'js2-mode-hook 'ac-js2-mode)`:
/// enabling the minor mode arms `completion-at-point`, the save hook, the
/// skewer hook and the three navigation keys, and completing a half-typed local
/// name inside a function inserts the declaration ac-js2 found in the parse
/// tree.
#[test]
fn ac_js2_mode_arms_completion_and_completes_a_local_name_at_point() {
    let elisp_form = r##"(ac-js2-test-in-source
 ac-js2-test-source
 (let ((mode (list :major major-mode
                   :minor (and ac-js2-mode t)
                   :capf (car completion-at-point-functions)
                   :save (and (memq 'ac-js2-save before-save-hook) t)
                   :skewer-hook (and (memq 'ac-js2-on-skewer-load skewer-js-hook) t)
                   :clients skewer-clients
                   :keys (mapcar (lambda (key) (lookup-key ac-js2-mode-map (kbd key)))
                                 '("M-." "M-," "C-c C-c"))
                   :errors (length js2-parsed-errors))))
   (goto-char (point-min))
   (search-forward "    greet(visitor")
   (goto-char (line-end-position))
   (insert "\n    sho")
   (let ((completion (ac-js2-test-completion)))
     (completion-at-point)
     (list :mode mode
           :completion completion
           :line (buffer-substring-no-properties (line-beginning-position)
                                                 (line-end-position))
           :point (point)))))"##;

    let expect = expect![[
        r#"OK (:mode (:major js2-mode :minor t :capf ac-js2-completion-function :save t :skewer-hook t :clients nil :keys (ac-js2-jump-to-definition pop-tag-mark ac-js2-expand-function) :errors 0) :completion (:beg 428 :end 431 :locals ("visitor" "main" "shout" "config" "greet" "greeting") :total 230 :point 431) :line "    shout" :point 433)"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

/// Every candidate ac-js2 offers from the buffer carries documentation built
/// from the parse tree: a named function shows its signature, a function held
/// in a variable shows its anonymous signature, an object literal is rendered
/// property by property, and a comment above a declaration is prepended.
#[test]
fn parse_tree_candidates_carry_signatures_object_shapes_and_leading_comments() {
    let elisp_form = r##"(ac-js2-test-in-source
 ac-js2-test-source
 (goto-char (point-max))
 (let* ((completion (ac-js2-test-completion))
        (docs (ac-js2-test-docs (plist-get completion :locals))))
   (list :completion completion :docs docs)))"##;

    let expect = expect![[
        r#"OK (:completion (:beg nil :end nil :locals ("main" "shout" "config" "greet" "greeting") :total 229 :point 452) :docs (("main" . "function main()") ("shout" . "function (text)") ("config" . "locale : \"de-DE\"\nretries : 3\nonError : function (error)") ("greet" . "Returns a polite greeting for NAME.\n\nfunction greet(name, punctuation)") ("greeting" . "Utilities for the demo app. */\n\"Grüße\"")))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

/// Typing a dot after an object switches ac-js2 to its property branch: with no
/// browser connected the candidates are exactly the keys of the object literal
/// the parser found, each documented with its initializer, and none of the
/// keyword or extern padding is added.
#[test]
fn dot_completion_lists_the_properties_of_a_real_object_literal() {
    let elisp_form = r##"(ac-js2-test-in-source
 ac-js2-test-source
 (goto-char (point-min))
 (search-forward "    greet(visitor, \"!\");")
 (goto-char (line-end-position))
 (insert "\n    config.")
 (let* ((completion (ac-js2-test-completion))
        (docs (ac-js2-test-docs (plist-get completion :locals))))
   (list :completion completion :docs docs)))"##;

    let expect = expect![[
        r#"OK (:completion (:beg 435 :end 435 :locals ("locale" "retries" "onError") :total 3 :point 435) :docs (("locale" . "\"de-DE\"") ("retries" . "3") ("onError" . "function (error)")))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

/// `M-.` on a call jumps to the function declaration and `M-.` on `config.locale`
/// jumps to the property inside the object literal, both to the exact character
/// position.  Two documented paths are broken against this Emacs: `M-,` cannot
/// return, because ac-js2 pushes onto `find-tag-marker-ring` while `pop-tag-mark`
/// now reads xref's history, and the same call makes ac-js2's own "No jump
/// location found" report unreachable for an undeclared name.
#[test]
fn jump_to_definition_lands_on_declarations_and_object_literal_properties() {
    let elisp_form = r##"(ac-js2-test-in-source
 ac-js2-test-source
 (let ((origin (ac-js2-test-point-in "    greet(visitor" "greet")))
   (execute-kbd-macro (kbd "M-."))
   (let ((jumped (list (point)
                       (buffer-substring-no-properties (point) (+ (point) 33))
                       (ring-length find-tag-marker-ring))))
     (let ((popped (condition-case error
                       (progn (execute-kbd-macro (kbd "M-,")) (list 'ok (point)))
                     (error (list (car error) (cdr error) (point))))))
       (ac-js2-test-point-in "shout(config.locale" "locale")
       (execute-kbd-macro (kbd "M-."))
       (let ((prop (list (point)
                         (buffer-substring-no-properties (point) (+ (point) 16))
                         (ring-length find-tag-marker-ring))))
         (goto-char (point-max))
         (insert "\nmissingThing;\n")
         (js2-reparse)
         (ac-js2-test-point-in "missingThing" "missingThing")
         (let ((missing (condition-case error
                            (progn (execute-kbd-macro (kbd "M-.")) (list 'ok (point)))
                          (error (list (car error) (cdr error) (point))))))
           (goto-char (point-min))
           (search-forward "\"Ada\"")
           (backward-char 3)
           (let ((unsupported (condition-case error
                                  (progn (execute-kbd-macro (kbd "M-.")) (list 'ok (point)))
                                (error (list (car error) (cdr error) (point))))))
             (list :origin origin :jumped jumped :popped popped :prop prop
                   :missing missing :unsupported unsupported
                   :ring (ring-length find-tag-marker-ring)))))))))"##;

    let expect = expect![[
        r#"OK (:origin 405 :jumped (99 "function greet(name, punctuation)" 1) :popped (user-error ("At start of xref history") 99) :prop (206 "locale: \"de-DE\"," 2) :missing (user-error ("At start of xref history") 455) :unsupported (error ("Node is not a supported jump node") 394) :ring 4)"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

/// With a browser connected through skewer, the properties it reports are
/// appended to the ones the parser found and their documentation is trimmed to
/// the function signature, while a local property keeps the documentation built
/// from the buffer.  Because `ac-js2-evaluate-calls' is off by default, an
/// expression containing a call is never sent to the browser at all.
#[test]
fn a_connected_browser_merges_its_properties_and_calls_stay_unevaluated() {
    let elisp_form = r##"(ac-js2-test-in-source
 ac-js2-test-source
 (let ((skewer-clients '(fake-browser))
       (ac-js2-evaluate-calls nil)
       requests)
   (cl-letf (((symbol-function 'skewer-eval-synchronously)
              (lambda (string &rest args)
                (push (list string (plist-get args :type) (plist-get args :extra))
                      requests)
                '((status . "success")
                  (value . [(locale . "\"de-DE\"")
                            (hasOwnProperty . "function hasOwnProperty() { [native code] }")
                            (toLocaleString . "function toLocaleString() { [native code] }")])))))
     (goto-char (point-min))
     (search-forward "    shout(config.locale);")
     (goto-char (line-end-position))
     (insert "\n    config.")
     (let* ((dot (ac-js2-completion-function))
            (docs (ac-js2-test-docs
                   '("locale" "retries" "hasOwnProperty" "toLocaleString"))))
       (delete-region (line-beginning-position) (point))
       (insert "    greet(visitor).")
       (js2-reparse)
       (let ((call (ac-js2-completion-function)))
         (list :dot (list :beg (nth 0 dot) :end (nth 1 dot) :candidates (nth 2 dot))
               :docs docs
               :call-candidates (nth 2 call)
               :requests (reverse requests)))))))"##;

    let expect = expect![[
        r#"OK (:dot (:beg 461 :end 461 :candidates ("locale" "retries" "onError" "locale" "hasOwnProperty" "toLocaleString")) :docs (("locale" . "\"de-DE\"") ("retries" . "3") ("hasOwnProperty" . "function hasOwnProperty()") ("toLocaleString" . "function toLocaleString()")) :call-candidates nil :requests (("config" "complete" ((prototypes . t)))))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

/// The two degenerate buffers.  An empty file still completes, offering only
/// the keyword and extern padding.  A file that does not parse -- the normal
/// state while a function signature is being typed -- makes the documented
/// `completion-at-point' entry point signal, because ac-js2 asks a recovered
/// node for an absolute position it does not have.
#[test]
fn an_empty_buffer_completes_the_externs_and_an_unparseable_one_signals() {
    let elisp_form = r##"(let ((empty (ac-js2-test-in-source
              ""
              (let ((result (ac-js2-completion-function)))
                (list :beg (nth 0 result) :end (nth 1 result)
                      :locals (ac-js2-test-local-candidates (nth 2 result))
                      :total (length (nth 2 result))
                      :first (car (nth 2 result))
                      :errors (length js2-parsed-errors))))))
  (let ((broken (ac-js2-test-in-source
                 "var ready = true;\nfunction broken(alpha {\n    return alpha;\n}\nvar tail = 1;\n"
                 (goto-char (point-max))
                 (list :errors (mapcar (lambda (entry)
                                         (list (car (car entry)) (cadr entry)))
                                       js2-parsed-errors)
                       :completion (condition-case error
                                       (ac-js2-completion-function)
                                     (error (list (car error) (cdr error))))
                       :point (point)
                       :text (buffer-substring-no-properties (point-min) (point-max))))))
    (list :empty empty :broken broken)))"##;

    let expect = expect![[
        r#"OK (:empty (:beg nil :end nil :locals nil :total 224 :first "break" :errors 0) :broken (:errors (("msg.no.paren.after.parms" 41)) :completion (wrong-type-argument (number-or-marker-p nil)) :point 77 :text "var ready = true;\nfunction broken(alpha {\n    return alpha;\n}\nvar tail = 1;\n"))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

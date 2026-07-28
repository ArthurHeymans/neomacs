use expect_test::expect;

use super::assert_actionscript_mode_parity;

/// The mode's front door: `auto-mode-alist' claims `.as' (case-folded, so
/// `Ticker.AS' too) while `.asc' and a backup name are left alone, and visiting
/// such a file gives the ActionScript environment.  This is an old-style major
/// mode - no `define-derived-mode', so it is not derived from `prog-mode' - and
/// everything it offers is the buffer-local state and the five keys asserted
/// here.  Note it never sets `comment-end'.
#[test]
fn visiting_an_as_file_sets_up_the_actionscript_editing_environment() {
    let elisp_form = r##"(let ((buffer (as-test-open "src/com/example/game/Ticker.as" as-test-ticker)))
  (unwind-protect
      (with-current-buffer buffer
        (list
         (list major-mode
               mode-name
               (and (derived-mode-p 'prog-mode) t)
               (and (derived-mode-p 'actionscript-mode) t)
               indent-line-function
               comment-start
               comment-end
               comment-start-skip
               parse-sexp-ignore-comments
               (car font-lock-defaults)
               actionscript-indent-level
               actionscript-font-lock-level)
         (list (eq (syntax-table) actionscript-mode-syntax-table)
               (eq (current-local-map) actionscript-mode-map)
               (key-binding (kbd "C-M-a"))
               (key-binding (kbd "C-M-e"))
               (key-binding (kbd "C-M-h"))
               (key-binding (kbd "C-c C-c"))
               (key-binding (kbd "C-c C-u")))
         (mapcar (lambda (name)
                   (cons name (assoc-default name auto-mode-alist #'string-match-p)))
                 '("Ticker.as" "Ticker.asc" "Ticker.as.bak" "Ticker.AS" "ticker.as~"))
         (list (buffer-modified-p) (point) (buffer-size))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ((actionscript-mode "Actionscript" nil t actionscript-indent-line "//" "" "\\(//+\\|/\\*+\\)\\s *" t (actionscript-font-lock-keywords-2) 4 2) (t t as-beginning-of-defun as-end-of-defun as-mark-defun comment-region uncomment-region) (("Ticker.as" . actionscript-mode) ("Ticker.asc") ("Ticker.as.bak" nil t) ("Ticker.AS" . actionscript-mode) ("ticker.as~")) (nil 1 682))"#
    ]];

    assert_actionscript_mode_parity(elisp_form, expect);
}

/// Commenting, the way a user does it: select two lines and press `M-;' to
/// comment them with the mode's `//', press `M-;' on the commented region to
/// take it back, then `M-;' at the end of a line to append a trailing comment
/// at `comment-column'.  The mode also binds `C-c C-c' and `C-c C-u' directly
/// to `comment-region' and `uncomment-region', which must agree.
#[test]
fn comment_dwim_round_trips_comments_through_the_mode_bindings() {
    let elisp_form = r##"(let ((buffer (as-test-open "src/Comments.as"
                            (concat "package {\n"
                                    "    public class Comments {\n"
                                    "        public function go():void {\n"
                                    "            var total:int = 0;\n"
                                    "            total += 1;\n"
                                    "            trace(total);\n"
                                    "        }\n"
                                    "    }\n"
                                    "}\n"))))
  (unwind-protect
      (with-current-buffer buffer
        (set-window-buffer (selected-window) buffer)
        (transient-mark-mode 1)
        (goto-char (as-test-at "            var total"))
        (execute-kbd-macro (kbd "C-SPC C-n C-n M-;"))
        (let ((commented (buffer-string)))
          (goto-char (as-test-at "// var total"))
          (goto-char (line-beginning-position))
          (execute-kbd-macro (kbd "C-SPC C-n C-n M-;"))
          (let ((uncommented (buffer-string)))
            (goto-char (as-test-at "total += 1;"))
            (execute-kbd-macro (kbd "C-e M-; k e e p"))
            (let ((appended (buffer-substring-no-properties
                             (line-beginning-position) (line-end-position))))
              (goto-char (as-test-at "            var total"))
              (execute-kbd-macro (kbd "C-SPC C-n C-c C-c"))
              (let ((region-commented (buffer-string)))
                (goto-char (as-test-at "// var total"))
                (goto-char (line-beginning-position))
                (execute-kbd-macro (kbd "C-SPC C-n C-c C-u"))
                (list commented uncommented appended region-commented
                      (buffer-string) (point) (buffer-modified-p)))))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ("package {\n    public class Comments {\n        public function go():void {\n            // var total:int = 0;\n            // total += 1;\n            trace(total);\n        }\n    }\n}\n" "package {\n    public class Comments {\n        public function go():void {\n            var total:int = 0;\n            total += 1;\n            trace(total);\n        }\n    }\n}\n" "            total += 1;\11\11//keep" "package {\n    public class Comments {\n        public function go():void {\n            // var total:int = 0;\n            total += 1;\11\11//keep\n            trace(total);\n        }\n    }\n}\n" "package {\n    public class Comments {\n        public function go():void {\n            var total:int = 0;\n            total += 1;\11\11//keep\n            trace(total);\n        }\n    }\n}\n" 106 t)"#
    ]];

    assert_actionscript_mode_parity(elisp_form, expect);
}

/// `indent-region' over the whole class, using the mode's own
/// `actionscript-indent-line'.  The interesting part is the second half: the
/// indenter asks `as3-face-at-point' whether a brace is inside a string or a
/// comment, so in a buffer that has been fontified the `}' inside the trace
/// string and the one in the trailing comment are skipped and the `else' branch
/// and the closing braces line up - while in a buffer that has not been
/// fontified the very same file collapses from that line onwards.
#[test]
fn indent_region_lays_out_a_class_and_needs_fontification_to_skip_braces() {
    let elisp_form = r##"(let ((fontified (as-test-open "src/Fontified.as" as-test-ticker))
      (plain (as-test-open "src/Plain.as" as-test-ticker)))
  (unwind-protect
      (list
       (with-current-buffer fontified
         (font-lock-ensure)
         (indent-region (point-min) (point-max))
         (list (buffer-substring-no-properties (point-min) (point-max))
               (buffer-modified-p)
               indent-tabs-mode
               tab-width))
       (with-current-buffer plain
         (indent-region (point-min) (point-max))
         (buffer-substring-no-properties (point-min) (point-max))))
    (kill-buffer fontified)
    (kill-buffer plain)))"##;
    let expect = expect![[
        r#"OK (("package com.example.game {\n\n    import flash.display.Sprite;\n    import flash.events.Event;\n\n    /**\n    * A sprite that counts frames.\n    */\n    public class Ticker extends Sprite implements ITickable {\n\n\11public static const MAX_TICKS:int = 100;\n\n\11private var _label:String = 'ready';\n\11private var _count:uint = 0;\n\n\11public function Ticker(label:String = \"ready\") {\n\11    _label = label;\n\11    addEventListener(Event.ENTER_FRAME, onEnterFrame);\n\11}\n\n\11public function get count():uint {\n\11    return _count;\n\11}\n\n\11private function onEnterFrame(event:Event):void {\n\11    if (_count < MAX_TICKS) {\n\11\11_count++;\n\11\11trace(\"tick } \" + _count);  // closing brace } in a comment\n\11    } else {\n\11\11removeEventListener(Event.ENTER_FRAME, onEnterFrame);\n\11    }\n\11}\n    }\n}\n" t t 8) "package com.example.game {\n\n    import flash.display.Sprite;\n    import flash.events.Event;\n\n    /**\n    * A sprite that counts frames.\n    */\n    public class Ticker extends Sprite implements ITickable {\n\n\11public static const MAX_TICKS:int = 100;\n\n\11private var _label:String = 'ready';\n\11private var _count:uint = 0;\n\n\11public function Ticker(label:String = \"ready\") {\n\11    _label = label;\n\11    addEventListener(Event.ENTER_FRAME, onEnterFrame);\n\11}\n\n\11public function get count():uint {\n\11    return _count;\n\11}\n\n\11private function onEnterFrame(event:Event):void {\n\11    if (_count < MAX_TICKS) {\n\11\11_count++;\n\11\11trace(\"tick } \" + _count);  // closing brace } in a comment\n    } else {\n\11removeEventListener(Event.ENTER_FRAME, onEnterFrame);\n    }\n}\n}\n}\n")"#
    ]];

    assert_actionscript_mode_parity(elisp_form, expect);
}

/// What the user actually sees at level 2 (the default
/// `actionscript-font-lock-level'): keywords, the leading package component,
/// the type at the end of an import, the class name and the names after
/// `extends'/`implements', function names, and both string flavours - AS3
/// allows `'single'' as well as `"double"' quotes - plus block and line
/// comments.  The built-in type names `int', `String' and `uint' land on
/// `font-lock-function-name-face' because the package lists them among its
/// global functions, and `MAX_TICKS' is left unfontified.
#[test]
fn font_lock_marks_packages_imports_classes_strings_and_comments() {
    let elisp_form = r##"(let ((buffer (as-test-open "src/Faces.as" as-test-ticker)))
  (unwind-protect
      (with-current-buffer buffer
        (goto-char (point-min))
        (list (as-test-face-runs (point-min) (as-test-at "public function Ticker"))
              (as-test-face-runs (as-test-at "private function onEnterFrame")
                                 (as-test-at "} else {"))
              (list (get-text-property (as-test-at "'ready'") 'face)
                    (get-text-property (as-test-at "\"tick }") 'face)
                    (get-text-property (as-test-at "// closing brace") 'face)
                    (get-text-property (as-test-at "* A sprite") 'face)
                    (get-text-property (as-test-at "MAX_TICKS:int") 'face))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ((("package" . font-lock-keyword-face) ("com" . font-lock-constant-face) ("import" . font-lock-keyword-face) ("flash" . font-lock-constant-face) ("display" . font-lock-constant-face) ("Sprite" . font-lock-type-face) ("import" . font-lock-keyword-face) ("flash" . font-lock-constant-face) ("events" . font-lock-constant-face) ("Event" . font-lock-type-face) ("/**" . font-lock-comment-delimiter-face) ("\n * A sprite that counts frames.\n */" . font-lock-comment-face) ("public" . font-lock-keyword-face) ("class" . font-lock-keyword-face) ("Ticker" . font-lock-type-face) ("extends" . font-lock-keyword-face) ("Sprite" . font-lock-type-face) ("implements" . font-lock-keyword-face) ("ITickable" . font-lock-type-face) ("public" . font-lock-keyword-face) ("static" . font-lock-keyword-face) ("const" . font-lock-keyword-face) ("int" . font-lock-function-name-face) ("private" . font-lock-keyword-face) ("var" . font-lock-keyword-face) ("String" . font-lock-function-name-face) ("'ready'" . font-lock-string-face) ("private" . font-lock-keyword-face) ("var" . font-lock-keyword-face) ("uint" . font-lock-function-name-face)) (("private" . font-lock-keyword-face) ("function" . font-lock-keyword-face) ("onEnterFrame" . font-lock-function-name-face) ("void" . font-lock-keyword-face) ("if" . font-lock-keyword-face) ("trace" . font-lock-function-name-face) ("\"tick } \"" . font-lock-string-face) ("// " . font-lock-comment-delimiter-face) ("closing brace } in a comment\n" . font-lock-comment-face)) (font-lock-string-face font-lock-string-face font-lock-comment-delimiter-face font-lock-comment-face nil))"#
    ]];

    assert_actionscript_mode_parity(elisp_form, expect);
}

/// The syntax table is what makes everything else work: `_' and `$' are word
/// constituents (so `forward-word' crosses `$mixed_name' in one go), `'' is a
/// string delimiter next to `"', and `/' plus `*' spell both comment styles, so
/// `syntax-ppss' reports the two string kinds, a block comment, a line comment
/// (style b) and the paren depth, and `forward-sexp' can cross a parenthesised
/// expression and a brace block.
#[test]
fn the_syntax_table_classifies_strings_comments_and_dollar_identifiers() {
    let elisp_form = r##"(let ((buffer (as-test-open "src/Syn.as" as-test-syntax-sample)))
  (unwind-protect
      (with-current-buffer buffer
        (list
         (mapcar #'char-syntax '(?_ ?$ ?\' ?\" ?+ ?/ ?* ?{ ?\n))
         (list (as-test-ppss (as-test-at "double \\\"" 2))
               (as-test-ppss (as-test-at "single {" 2))
               (as-test-ppss (as-test-at "block {" 2))
               (as-test-ppss (as-test-at "line {" 2))
               (as-test-ppss (as-test-at "b || c"))
               (as-test-ppss (as-test-at "trace(\"x\")")))
         (progn (goto-char (as-test-at "$mixed_name"))
                (list (progn (forward-word) (point))
                      (buffer-substring-no-properties (as-test-at "$mixed_name") (point))))
         (progn (goto-char (as-test-at "(a &&"))
                (forward-sexp)
                (list (point) (buffer-substring-no-properties (as-test-at "(a &&") (point))))
         (progn (goto-char (as-test-at "{ trace"))
                (forward-sexp)
                (list (point) (buffer-substring-no-properties (- (point) 3) (point))))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ((119 119 34 34 46 46 46 40 62) ((:depth 2 :in-string 34 :in-comment nil :comment-style nil :start 48 :innermost-open 21) (:depth 2 :in-string 39 :in-comment nil :comment-style nil :start 87 :innermost-open 21) (:depth 2 :in-string nil :in-comment t :comment-style nil :start 106 :innermost-open 21) (:depth 3 :in-string nil :in-comment t :comment-style 1 :start 185 :innermost-open 148) (:depth 5 :in-string nil :in-comment nil :comment-style nil :start nil :innermost-open 159) (:depth 4 :in-string nil :in-comment nil :comment-style nil :start nil :innermost-open 169)) (38 "$mixed_name") (168 "(a && (b || c))") (184 "; }"))"#
    ]];

    assert_actionscript_mode_parity(elisp_form, expect);
}

/// The three motion keys the mode binds.  From inside the innermost `if',
/// `C-M-a' backs up to the enclosing function signature and `C-M-e' moves past
/// its closing brace; `C-M-h' marks a whole accessor, leaving point at its
/// start and the mark after its final brace.  From the top of the file, where
/// point is inside no function at all, `C-M-e' takes the other route and stops
/// after the first function in the buffer.
#[test]
fn defun_motion_commands_walk_actionscript_functions() {
    let elisp_form = r##"(let ((buffer (as-test-open "src/Motion.as" as-test-ticker)))
  (unwind-protect
      (with-current-buffer buffer
        (set-window-buffer (selected-window) buffer)
        (goto-char (as-test-at "_count++;"))
        (execute-kbd-macro (kbd "C-M-a"))
        (let ((beginning (list (point)
                               (line-number-at-pos)
                               (buffer-substring-no-properties
                                (line-beginning-position) (line-end-position)))))
          (execute-kbd-macro (kbd "C-M-e"))
          (let ((end (list (point)
                           (line-number-at-pos)
                           (buffer-substring-no-properties
                            (line-beginning-position) (point)))))
            (goto-char (as-test-at "return _count;"))
            (execute-kbd-macro (kbd "C-M-h"))
            (list beginning
                  end
                  (list (point)
                        (mark t)
                        (buffer-substring-no-properties (point) (mark t)))
                  (progn (goto-char (point-min))
                         (execute-kbd-macro (kbd "C-M-e"))
                         (list (point) (line-number-at-pos)))
                  (buffer-modified-p)))))
    (kill-buffer buffer)))"##;
    let expect = expect![[
        r#"OK ((466 25 "private function onEnterFrame(event:Event):void {") (678 32 "}") (413 464 "public function get count():uint {\nreturn _count;\n}") (411 19) nil)"#
    ]];

    assert_actionscript_mode_parity(elisp_form, expect);
}

/// The package ships an imenu expression and an initialiser but never calls
/// them - its own commentary lists imenu as a TODO - so out of the box the
/// buffer cannot build an index at all.  Adding the package's `as-imenu-init'
/// to `actionscript-mode-hook', which is what the pieces are there for, gives a
/// full index of the constructor, the accessor and the private method, turns
/// case folding off as the initialiser promises, and lets `imenu' jump.
#[test]
fn imenu_indexes_functions_once_wired_with_the_packages_own_helper() {
    let elisp_form = r##"(progn
 (require 'imenu)
 (let ((bare (as-test-open "src/Bare.as" as-test-ticker)))
  (unwind-protect
      (let ((unwired
             (with-current-buffer bare
               (list imenu-generic-expression
                     imenu-case-fold-search
                     (condition-case error (progn (imenu--make-index-alist) :indexed)
                       (error error))))))
        (let ((actionscript-mode-hook
               (list (lambda () (as-imenu-init as-imenu-generic-expression)))))
          (let ((wired (as-test-open "src/Wired.as" as-test-ticker)))
            (unwind-protect
                (with-current-buffer wired
                  (let ((index (imenu--make-index-alist)))
                    (list unwired
                          imenu-case-fold-search
                          (mapcar (lambda (entry)
                                    (if (markerp (cdr entry))
                                        (cons (car entry) (marker-position (cdr entry)))
                                      entry))
                                  index)
                          (progn (goto-char (point-max))
                                 (imenu (assoc "onEnterFrame" index))
                                 (list (point)
                                       (buffer-substring-no-properties
                                        (point) (line-end-position)))))))
              (kill-buffer wired)))))
    (kill-buffer bare))))"##;
    let expect = expect![[
        r#"OK ((nil t (imenu-unavailable "This buffer cannot use ‘imenu-default-create-index-function’")) nil (("*Rescan*" . -99) ("Ticker" . 294) ("count" . 413) ("onEnterFrame" . 466)) (466 "private function onEnterFrame(event:Event):void {"))"#
    ]];

    assert_actionscript_mode_parity(elisp_form, expect);
}

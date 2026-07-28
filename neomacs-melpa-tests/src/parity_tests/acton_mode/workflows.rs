use expect_test::expect;

use super::assert_acton_mode_parity;

/// Opening an Acton source file has to give the reader a buffer that is already
/// set up for the language.  Six real file names pin which ones the package's
/// `auto-mode-alist' entry claims -- note that it claims `.act' and not
/// `.acton', and that `.py' still belongs to `python-mode' -- and the resulting
/// buffer pins the editing defaults the mode installs: the `#' comment syntax,
/// its own line indenter, a tab width taken from `acton-indent-offset', spaces
/// rather than tabs, its syntax table, and the colon hook that realigns
/// `else'/`elif'/`except'/`finally'.
#[test]
fn visiting_an_acton_file_selects_the_mode_and_its_editing_defaults() {
    let elisp_form = r##"(progn
  (let ((observed nil))
    (dolist (name '("counter.act" "counter.ACT" "counter.act.bak" "counter.acton"
                    "counter.py" "act"))
      (let ((buffer (actn-test-visit name "actor Counter():\n    pass\n")))
        (push (list name major-mode) observed)
        (kill-buffer buffer)))
    (actn-test-visit "counter.act" actn-test-counter)
    (list :routing (nreverse observed)
          :alist (assoc "\\.act\\'" auto-mode-alist)
          :mode major-mode
          :mode-name mode-name
          :parent (get 'acton-mode 'derived-mode-parent)
          :comment (list comment-start comment-start-skip comment-column
                         comment-use-syntax)
          :indent (list indent-line-function tab-width indent-tabs-mode
                        acton-indent-offset)
          :colon-hook (and (memq 'acton-handle-colon post-self-insert-hook) t)
          :syntax-table (eq (syntax-table) acton-mode-syntax-table)
          :paragraph (equal paragraph-start paragraph-separate))))"##;

    let expect = expect![[
        r##"OK (:routing (("counter.act" acton-mode) ("counter.ACT" acton-mode) ("counter.act.bak" acton-mode) ("counter.acton" fundamental-mode) ("counter.py" python-mode) ("act" fundamental-mode)) :alist ("\\.act\\'" . acton-mode) :mode acton-mode :mode-name "Acton" :parent prog-mode :comment ("#" "#+\\s-*" 40 t) :indent (acton-indent-line 4 nil 4) :colon-hook t :syntax-table t :paragraph t)"##
    ]];

    assert_acton_mode_parity(elisp_form, expect);
}

/// Reading a realistic actor: an import, an `actor' with a typed parameter and
/// a hexadecimal literal, an `action def' with an arrow return type, an `if'
/// with a comparison and a string, then a `class' and a `protocol'.  The
/// complete run-by-run face map pins every category the mode distinguishes and
/// their precedence -- declaration keywords, the effect qualifier `action' as a
/// builtin, capitalised names after `actor'/`class'/`protocol' and after a `:'
/// annotation as types, the function name after `def', numbers, operators and
/// the comment split into its delimiter and body.  Visiting must not modify the
/// buffer or move point.
#[test]
fn syntax_highlighting_covers_declarations_effects_types_and_literals() {
    let elisp_form = r##"(progn
  (actn-test-visit "counter.act" actn-test-counter)
  (list :runs (actn-test-face-runs)
        :point (point)
        :modified (buffer-modified-p)))"##;

    let expect = expect![[
        r##"OK (:runs (("# " . font-lock-comment-delimiter-face) ("a counter actor, from the Acton tutorial\n" . font-lock-comment-face) ("import" . font-lock-keyword-face) (" acton.rts\n\n") ("actor" . font-lock-keyword-face) (" ") ("Counter" . font-lock-type-face) ("(name: str):\n    ") ("var" . font-lock-keyword-face) (" count = ") ("0" . font-lock-constant-face) ("\n    limit: ") ("Int" . font-lock-type-face) (" = ") ("0x10" . font-lock-constant-face) ("\n\n    ") ("action" . font-lock-builtin-face) (" ") ("def" . font-lock-keyword-face) (" ") ("bump" . font-lock-function-name-face) ("(step: int) ") ("->" . font-lock-builtin-face) (" int:\n        count ") ("+" . font-lock-builtin-face) ("= step\n        ") ("if" . font-lock-keyword-face) (" count ") (">" . font-lock-builtin-face) (" limit:\n            print(") ("\"over \"" . font-lock-string-face) (", name)\n        ") ("return" . font-lock-keyword-face) (" count\n\n") ("class" . font-lock-keyword-face) (" ") ("Point" . font-lock-type-face) ("(object):\n    ") ("def" . font-lock-keyword-face) (" ") ("__init__" . font-lock-function-name-face) ("(self, x: float):\n        self.x = x\n\n") ("protocol" . font-lock-keyword-face) (" ") ("Drawable" . font-lock-type-face) (":\n    ") ("def" . font-lock-keyword-face) (" ") ("draw" . font-lock-function-name-face) ("(self) ") ("->" . font-lock-builtin-face) (" ") ("None" . font-lock-constant-face) (":\n        ") ("pass" . font-lock-keyword-face) ("\n")) :point 1 :modified nil)"##
    ]];

    assert_acton_mode_parity(elisp_form, expect);
}

/// Reindenting a body someone pasted in flush left.  The mode's indenter works
/// from the previous non-blank line: a line ending in `:' opens a level, a line
/// starting with `return'/`pass'/`break'/`continue'/`raise' closes one, and
/// anything else keeps the current level -- which is why the `def other()' that
/// follows a `return' lands one level in rather than back at the top.  Spaces
/// are used throughout, never tabs.  The same file is indented twice, once
/// fontified and once not, to pin that the indenter reads syntax rather than
/// faces and gives byte-identical output either way.
#[test]
fn reindenting_a_body_follows_colons_and_block_enders_without_fontification() {
    let elisp_form = r##"(progn
  (actn-test-visit "indent.act" actn-test-unindented)
  (indent-region (point-min) (point-max))
  (let ((plain (actn-test-text)))
    (actn-test-visit "indent-fontified.act" actn-test-unindented)
    (font-lock-ensure)
    (indent-region (point-min) (point-max))
    (list :plain plain
          :fontified (actn-test-text)
          :same (equal plain (actn-test-text))
          :tabs (and (string-match-p "\t" plain) t))))"##;

    let expect = expect![[
        r##"OK (:plain "actor Counter():\nvar count = 0\ndef bump(step: int) -> int:\n    count += step\n    if count > 10:\n        print(\"over\")\n        return count\n    def other():\n        pass\n" :fontified "actor Counter():\nvar count = 0\ndef bump(step: int) -> int:\n    count += step\n    if count > 10:\n        print(\"over\")\n        return count\n    def other():\n        pass\n" :same t :tabs nil)"##
    ]];

    assert_acton_mode_parity(elisp_form, expect);
}

/// The mode's one interactive behaviour, driven by really typing it: with point
/// left one level too deep after a `return', typing `e l s e :' must pull the
/// new line back to the indentation of the `if' it belongs to as soon as the
/// colon lands, leaving point after the colon on the realigned line.
#[test]
fn typing_else_realigns_the_line_with_its_if() {
    let elisp_form = r##"(progn
  (actn-test-visit "colon.act"
    "def check(x: int) -> int:\n    if x > 0:\n        return x\n        ")
  (goto-char (point-max))
  (let ((before (actn-test-text)))
    (execute-kbd-macro (kbd "e l s e :"))
    (list :before before
          :after (actn-test-text)
          :line (line-number-at-pos)
          :column (current-column)
          :modified (buffer-modified-p))))"##;

    let expect = expect![[
        r##"OK (:before "def check(x: int) -> int:\n    if x > 0:\n        return x\n        " :after "def check(x: int) -> int:\n    if x > 0:\n        return x\n    else:" :line 4 :column 9 :modified t)"##
    ]];

    assert_acton_mode_parity(elisp_form, expect);
}

/// Commenting out a block of a real Acton file and putting it back.  Because
/// the mode declares `#' as a genuine comment syntax rather than only painting
/// it, `comment-region' prefixes each line and `uncomment-region' restores the
/// file byte for byte, and the mode's own comment line is fontified as a
/// comment.
#[test]
fn commenting_a_block_round_trips_through_the_modes_comment_syntax() {
    let elisp_form = r##"(progn
  (actn-test-visit "comment.act" actn-test-counter)
  (goto-char (point-min))
  (forward-line 3)
  (let* ((beg (line-beginning-position))
         (end (progn (forward-line 2) (line-end-position)))
         (commented (progn (comment-region beg end)
                           (buffer-substring-no-properties
                            (point-min) (point-max)))))
    (uncomment-region beg (line-end-position))
    (list :commented-block (let ((lines (split-string commented "\n")))
                             (list (nth 3 lines) (nth 4 lines) (nth 5 lines)))
          :restored (equal (actn-test-text) actn-test-counter)
          :comment-face (save-excursion
                          (goto-char (point-min))
                          (font-lock-ensure)
                          (search-forward "counter actor")
                          (get-text-property (1- (point)) 'face)))))"##;

    let expect = expect![[
        r##"OK (:commented-block ("# actor Counter(name: str):" "#     var count = 0" "#     limit: Int = 0x10") :restored t :comment-face font-lock-comment-face)"##
    ]];

    assert_acton_mode_parity(elisp_form, expect);
}

/// Navigating the file: the mode's imenu index groups the actor, the class, the
/// protocol and the plain `def's, each at the position of its name.  The gap
/// this pins is real -- the index's function pattern anchors `def' to the start
/// of the line, so `action def bump' is not listed at all -- as is the
/// distinction the syntax table draws between a real comment, a real string and
/// ordinary code.
#[test]
fn imenu_indexes_the_declarations_and_the_syntax_table_classifies_text() {
    let elisp_form = r##"(progn
  (actn-test-visit "counter.act" actn-test-counter)
  (list :imenu (actn-test-imenu)
        :in-comment (save-excursion (goto-char (point-min))
                                    (search-forward "counter actor")
                                    (list (nth 4 (syntax-ppss)) (nth 8 (syntax-ppss))))
        :in-string (save-excursion (goto-char (point-min))
                                   (search-forward "over")
                                   (nth 3 (syntax-ppss)))
        :in-code (save-excursion (goto-char (point-min))
                                 (search-forward "count += step")
                                 (list (nth 3 (syntax-ppss)) (nth 4 (syntax-ppss))))))"##;

    let expect = expect![[
        r##"OK (:imenu (("Function" ("__init__" . 292) ("draw" . 365)) ("Protocol" ("Drawable" . 346)) ("Actor" ("Counter" . 62)) ("Class" ("Point" . 271))) :in-comment (t 1) :in-string 34 :in-code (nil nil))"##
    ]];

    assert_acton_mode_parity(elisp_form, expect);
}

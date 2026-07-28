use expect_test::expect;

use super::assert_ample_regexps_parity;

/// One `define-arx' call is the whole installation: it leaves behind a macro, a
/// `-to-string' function for building the same regexp at run time, a
/// `-bindings' variable holding the translated forms, and the properties that
/// mark them as belonging to this arx.  The regexp a realistic composition
/// produces is pinned whole, from both the macro and the function, and the
/// bindings are pinned as the package translated them.
#[test]
fn define_arx_builds_the_macro_its_to_string_function_and_its_bindings() {
    let elisp_form = r##"(progn
  (arx-test-define-log-rx)
  (list :surface (arx-test-surface 'log-rx)
        :bindings log-rx-bindings
        :log-line (arx-test-expand '(log-rx stamp ws level ws qualified))
        :same-from-to-string (log-rx-to-string '(seq stamp ws level ws qualified) t)
        :macro-and-function-agree
        (equal (arx-test-expand '(log-rx stamp ws level ws qualified))
               (log-rx-to-string '(seq stamp ws level ws qualified) t))))"##;

    let expect = expect![[
        r#"OK (:surface (:macro t :to-string t :bindings-bound t :arx-name "log-rx" :to-string-arx-name "log-rx" :form-count 6) :bindings ((ws (regexp "[ \11]+")) (level (or "DEBUG" "INFO" "WARN" "ERROR")) (ident (regexp "[A-Za-z_][A-Za-z0-9_]*")) (qualified (seq ident (* "." ident))) (stamp (seq (= 4 digit) "-" (= 2 digit) "-" (= 2 digit))) (bracketed (&rest bracketed-args) (eval (arx--apply-func-post-27 '(1 2) nil #[(form &rest args) "��������\10B��BBB��\"��" [args rx-to-string seq "[" ("]") t] 5] 'bracketed '(bracketed-args))))) :log-line "[[:digit:]]\\{4\\}-[[:digit:]]\\{2\\}-[[:digit:]]\\{2\\}\\(?:[ \11]+\\)\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)\\(?:[ \11]+\\)\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\(?:\\.\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\)*" :same-from-to-string "[[:digit:]]\\{4\\}-[[:digit:]]\\{2\\}-[[:digit:]]\\{2\\}\\(?:[ \11]+\\)\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)\\(?:[ \11]+\\)\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\(?:\\.\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\)*" :macro-and-function-agree t)"#
    ]];

    assert_ample_regexps_parity(elisp_form, expect);
}

/// The point of the library is composition, so this pins what each named form
/// contributes on its own, that a form defined in terms of an earlier one
/// (`qualified' over `ident') expands through it, and that named forms mix
/// freely with the `rx' forms that pass through untouched -- grouping,
/// alternation, character classes, anchors and the repetition and arity
/// constructs.  Each regexp is pinned whole.
#[test]
fn named_forms_compose_with_each_other_and_with_plain_rx_forms() {
    let elisp_form = r##"(progn
  (arx-test-define-log-rx)
  (list
   :each-form
   (mapcar (lambda (form) (cons form (log-rx-to-string form t)))
           '(ws level ident qualified stamp))
   :composition
   (list :grouped (arx-test-expand '(log-rx (group level) ": " (group qualified)))
         :alternation (arx-test-expand '(log-rx (or level ident)))
         :anchored (arx-test-expand '(log-rx line-start stamp ws level line-end))
         :repetition (arx-test-expand '(log-rx (one-or-more ident)
                                               (zero-or-one ws)
                                               (= 3 level)
                                               (** 1 4 ident)
                                               (repeat 2 stamp)))
         :classes (arx-test-expand '(log-rx (any "a-z" ?_) (not (any digit)) word-boundary))
         :nested (arx-test-expand '(log-rx (seq (or (seq stamp ws) (seq level ws))
                                                (zero-or-more qualified)))))))"##;

    let expect = expect![[
        r#"OK (:each-form ((ws . "[ \11]+") (level . "\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)") (ident . "[A-Za-z_][A-Za-z0-9_]*") (qualified . "\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\(?:\\.\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\)*") (stamp . "[[:digit:]]\\{4\\}-[[:digit:]]\\{2\\}-[[:digit:]]\\{2\\}")) :composition (:grouped "\\(\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)\\): \\(\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\(?:\\.\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\)*\\)" :alternation "\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)\\|[A-Za-z_][A-Za-z0-9_]*" :anchored "^[[:digit:]]\\{4\\}-[[:digit:]]\\{2\\}-[[:digit:]]\\{2\\}\\(?:[ \11]+\\)\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)$" :repetition "\\(?:[A-Za-z_][A-Za-z0-9_]*\\)+\\(?:[ \11]+\\)?\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)\\{3\\}\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\{1,4\\}\\(?:[[:digit:]]\\{4\\}-[[:digit:]]\\{2\\}-[[:digit:]]\\{2\\}\\)\\{2\\}" :classes "[_a-z][^[:digit:]]\\b" :nested "\\(?:[[:digit:]]\\{4\\}-[[:digit:]]\\{2\\}-[[:digit:]]\\{2\\}\\(?:[ \11]+\\)\\|\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)\\(?:[ \11]+\\)\\)\\(?:\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\(?:\\.\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\)*\\)*"))"#
    ]];

    assert_ample_regexps_parity(elisp_form, expect);
}

/// A `:func' form runs the user's function while the macro expands, with the
/// form's own arguments, and the arity range declared beside it is enforced --
/// too few arguments and too many both signal, with the message naming the
/// form, and a name the caller never defined is reported as an unknown rx
/// symbol rather than silently ignored.  `:predicate' is documented to filter
/// the form's arguments but the post-27 implementation accepts it and never
/// consults it, so an argument it should reject expands anyway -- and named as
/// a symbol rather than an evaluated function it has the same spelling problem
/// as `:func', failing at every use.
#[test]
fn func_forms_run_while_expanding_and_their_arity_and_predicate_are_enforced() {
    let elisp_form = r##"(progn
  (arx-test-define-log-rx)
  (eval '(define-arx pred-rx
           `((tagged (:func ,(lambda (form &rest args)
                               (rx-to-string `(seq "<" (seq ,@args) ">") t))
                            :predicate ,(symbol-function 'stringp)))))
        t)
  (eval '(define-arx symbol-pred-rx
           `((tagged (:func ,(lambda (form &rest args)
                               (rx-to-string `(seq "<" (seq ,@args) ">") t))
                            :predicate stringp))))
        t)
  (list :one-argument (arx-test-expand '(log-rx (bracketed level)))
        :two-arguments (arx-test-expand '(log-rx (bracketed level ws)))
        :inside-a-composition
        (arx-test-expand '(log-rx line-start (bracketed level) ws qualified))
        :too-few (arx-test-expand '(log-rx (bracketed)))
        :too-many (arx-test-expand '(log-rx (bracketed level ws level)))
        :unknown-form (arx-test-expand '(log-rx nosuchform))
        :predicate-satisfied (arx-test-expand '(pred-rx (tagged "a" "b")))
        :predicate-violated (arx-test-expand '(pred-rx (tagged 42)))
        :predicate-named-as-a-symbol
        (arx-test-expand '(symbol-pred-rx (tagged "a")))))"##;

    let expect = expect![[
        r#"OK (:one-argument "\\[\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)]" :two-arguments "\\[\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)\\(?:[ \11]+\\)]" :inside-a-composition "^\\(?:\\[\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)]\\)\\(?:[ \11]+\\)\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\(?:\\.\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\)*" :too-few (error "rx form ‘bracketed’ requires at least 1 arg") :too-many (error "rx form ‘bracketed’ accepts at most 2 args") :unknown-form (error "Unknown rx symbol ‘nosuchform’") :predicate-satisfied "<ab>" :predicate-violated "<\\*>" :predicate-named-as-a-symbol (void-variable stringp))"#
    ]];

    assert_ample_regexps_parity(elisp_form, expect);
}

/// These are macros, so the interesting question is what happens when the code
/// using them is byte-compiled: the expansion runs at compile time and the
/// regexp is baked into the `.elc'.  A real file in the sandbox defines its own
/// arx and two functions that use it, is compiled and loaded, and the regexps
/// its compiled functions return are compared with the ones the same forms
/// produce interpreted.
#[test]
fn the_generated_macro_produces_the_same_regexps_when_byte_compiled() {
    let elisp_form = r##"(let ((source (arx-test-write
                "lib/logmatch.el"
                (concat ";;; logmatch.el --- fixture  -*- lexical-binding: t; -*-\n"
                        "(require 'ample-regexps)\n"
                        "(define-arx bc-rx\n"
                        "  `((ws (regexp \"[ \\t]+\"))\n"
                        "    (level (or \"DEBUG\" \"INFO\" \"WARN\" \"ERROR\"))\n"
                        "    (ident (regexp \"[A-Za-z_][A-Za-z0-9_]*\"))\n"
                        "    (qualified (seq ident (* \".\" ident)))\n"
                        "    (wrapped (:func ,(lambda (form &rest args)\n"
                        "                       (rx-to-string `(seq \"<\" (seq ,@args) \">\") t))))))\n"
                        "(defun bc-line-regexp ()\n"
                        "  (bc-rx line-start level ws qualified line-end))\n"
                        "(defun bc-wrapped-regexp () (bc-rx (wrapped ident)))\n"
                        "(provide 'logmatch)\n"))))
  (require 'bytecomp)
  (let ((compiled (let ((byte-compile-verbose nil)
                        (byte-compile-warnings nil))
                    (byte-compile-file source))))
    (load (concat source "c") nil t t)
    (list :compiled compiled
          :elc-exists (file-exists-p (concat source "c"))
          :functions-are-byte-code
          (list (byte-code-function-p (symbol-function 'bc-line-regexp))
                (byte-code-function-p (symbol-function 'bc-wrapped-regexp)))
          :surface (arx-test-surface 'bc-rx)
          :line (bc-line-regexp)
          :wrapped (bc-wrapped-regexp)
          :matches-interpreted
          (list (equal (bc-line-regexp)
                       (bc-rx-to-string '(seq line-start level ws qualified line-end) t))
                (equal (bc-wrapped-regexp)
                       (bc-rx-to-string '(wrapped ident) t))))))"##;

    let expect = expect![[
        r#"OK (:compiled t :elc-exists t :functions-are-byte-code (t t) :surface (:macro t :to-string t :bindings-bound t :arx-name "bc-rx" :to-string-arx-name "bc-rx" :form-count 5) :line "^\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)\\(?:[ \11]+\\)\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\(?:\\.\\(?:[A-Za-z_][A-Za-z0-9_]*\\)\\)*$" :wrapped "<\\(?:[A-Za-z_][A-Za-z0-9_]*\\)>" :matches-interpreted (t t))"#
    ]];

    assert_ample_regexps_parity(elisp_form, expect);
}

/// The generated macro documents itself: `C-h f' on it lists every named form
/// with what it expands to, and points at the `-to-string' function for run-time
/// use.  Pinned whole, because this docstring is the only place a user finds out
/// what forms their arx offers.
#[test]
fn the_generated_macro_documents_every_named_form_it_offers() {
    let elisp_form = r##"(progn
  (arx-test-define-log-rx)
  (list :macro-documentation (documentation 'log-rx)
        :to-string-documentation (documentation 'log-rx-to-string)
        :bindings-documentation (get 'log-rx-bindings 'variable-documentation)))"##;

    let expect = expect![[
        r#"OK (:macro-documentation "Translate regular expressions REGEXPS in sexp form to a regexp string.\n\nSee macro ‘rx’ for more documentation on REGEXPS parameter.\nThis macro additionally supports the following forms:\n\n‘ws’\n    An alias for (regexp \"[ \\11]+\").\n\n‘level’\n    An alias for (or \"DEBUG\" \"INFO\" \"WARN\" \"ERROR\").\n\n‘ident’\n    An alias for (regexp \"[A-Za-z_][A-Za-z0-9_]*\").\n\n‘qualified’\n    An alias for (seq ident (* \".\" ident)).\n\n‘stamp’\n    An alias for (seq (= 4 digit) \"-\" (= 2 digit) \"-\" (= 2 digit)).\n\n‘(bracketed &rest args)’\n    Function without documentation.\n\nUse function ‘log-rx-to-string’ to do such a translation at run-time." :to-string-documentation "Parse and produce code for regular expression FORM.\n\nFORM is a regular expression in sexp form as supported by ‘log-rx’.\nNO-GROUP non-nil means don’t put shy groups around the result." :bindings-documentation "List of bindings for `log-rx' and `log-rx-to-string' functions.\n\nSee `log-rx' for a human readable list of defined forms.\n\nSee parameter BINDINGS for function `rx-let' for more information\nabout format of elements of this list.")"#
    ]];

    assert_ample_regexps_parity(elisp_form, expect);
}

/// Three parts of the package no longer work on a current Emacs, all pinned as
/// they behave.  `arx-and' and `arx-or', the helpers the docstring points at for
/// writing `:func' forms, call `rx-and'/`rx-or', which the rewritten `rx' no
/// longer has.  `arx-builder' reads a `NAME-constituents' variable that the
/// post-27 `define-arx' does not create.  And of the three ways the docstring
/// suggests naming a `:func' function, only an evaluated lambda works: a bare
/// symbol is accepted at definition time and then fails at every use, because
/// the generated binding embeds the symbol where a value is expected, and
/// `#'symbol' is rejected outright.
#[test]
fn the_func_helpers_and_the_builder_no_longer_work_on_a_current_emacs() {
    let elisp_form = r##"(progn
  (arx-test-define-log-rx)
  (list
   :arx-and (condition-case failure (arx-and '("a" "b")) (error failure))
   :arx-or (condition-case failure (arx-or '("a" "b")) (error failure))
   :arx-builder (condition-case failure (arx-builder "log-rx") (error failure))
   :symbol-func
   (list :defining (arx-test-expand '(define-arx symbol-rx
                                       '((wrapped (:func arx-test-wrap)))))
         :using (arx-test-expand '(symbol-rx (wrapped "x"))))
   :sharp-quoted-func
   (arx-test-expand '(define-arx sharp-rx
                       '((wrapped (:func #'arx-test-wrap)))))
   :lambda-func-works (arx-test-expand '(log-rx (bracketed level)))))"##;

    let expect = expect![[
        r#"OK (:arx-and (void-function rx-and) :arx-or (void-function rx-or) :arx-builder (void-variable log-rx-constituents) :symbol-func (:defining symbol-rx :using (void-variable arx-test-wrap)) :sharp-quoted-func (error "Not a function: #'arx-test-wrap") :lambda-func-works "\\[\\(?:DEBUG\\|ERROR\\|INFO\\|WARN\\)]")"#
    ]];

    assert_ample_regexps_parity(elisp_form, expect);
}

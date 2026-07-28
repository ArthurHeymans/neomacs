use expect_test::expect;

use super::assert_asn1_mode_parity;

/// The first thing that happens to an ASN.1 file: it is visited, the name
/// selects the mode, and the module gets re-indented.
///
/// The mode's own editing contract is asserted alongside the indentation
/// because the two are connected -- `tab-width' decides how the tabs SMIE
/// emits are read back, and `comment-start' decides what `comment-region'
/// will insert in the workflow below.
///
/// Indenting twice asserts a fixed point.  That is worth pinning separately
/// from the text: an indentation engine that is merely wrong is usable, one
/// that moves the line further every time it is asked is not, and the
/// difference is invisible in a single pass.
#[test]
fn visiting_an_asn1_file_selects_the_mode_and_indents_the_whole_module() {
    let elisp_form = r##"
(let ((buffer (asn1-test-visit "spec/bestellung.asn1" asn1-test-module)))
  (indent-region (point-min) (point-max))
  (let ((once (asn1-test-text)))
    (indent-region (point-min) (point-max))
    (list :mode major-mode
          :mode-name mode-name
          :parent (get major-mode 'derived-mode-parent)
          :tab-width tab-width
          :comment-start comment-start
          :comment-end comment-end
          :outline-regexp outline-regexp
          :outline-level outline-level
          :indented once
          :stable-under-reindent (equal once (asn1-test-text)))))
"##;

    let expect = expect![[
        r#"OK (:mode asn1-mode :mode-name "ASN.1" :parent prog-mode :tab-width 4 :comment-start "--" :comment-end "" :outline-regexp "-- +[0-9]+\\(\\.[0-9]+\\)* " :outline-level asn1-mode-outline-level :indented "-- 1 Bestellsystem\n-- 1.1 Grundtypen\nBestellung DEFINITIONS AUTOMATIC TAGS ::=\n\11BEGIN\n\11IMPORTS\n\11\11Kunde, Adresse\n\11\11FROM Kundenverwaltung;\n\11Auftrag ::= SEQUENCE {\n\11\11\11\11\11\11 nummer INTEGER (1..65535),\n\11kunde Kunde,\n\11posten Postenliste,\n\11hinweis UTF8String OPTIONAL\n\11}\n\11Postenliste ::= SEQUENCE SIZE (1..99) OF Posten\n\11\11\11\11\11\11\11\11\11\11\11 -- 1.2 Zustände\n\11\11\11\11\11\11\11\11\11\11\11 Zustand ::= ENUMERATED {\n\11\11\11\11\11\11\11\11\11\11\11\11\11\11\11\11\11offen (0),\n\11\11\11\11\11\11\11\11\11\11\11 versandt (1),\n\11\11\11\11\11\11\11\11\11\11\11 storniert (2)\n\11\11\11\11\11\11\11\11\11\11\11 }\n\11\11\11\11\11\11\11\11\11\11\11 -- 1.2.1 Zahlungsarten\n\11\11\11\11\11\11\11\11\11\11\11 Zahlung ::= CHOICE {\n\11\11\11\11\11\11\11\11\11\11\11\11\11\11\11\11rechnung Rechnung,\n\11\11\11\11\11\11\11\11\11\11\11 lastschrift Lastschrift\n\11\11\11\11\11\11\11\11\11\11\11 }\n\11\11\11\11\11\11\11\11\11\11\11 standardHinweis UTF8String ::= \"Grüße aus München\"\n\11END\n" :stable-under-reindent t)"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

/// The indentation above is ragged, and this narrows down why, because a
/// reader of the expectation above will otherwise assume the fixture is at
/// fault.
///
/// Three modules differing in one line: no `SEQUENCE OF' assignment, a plain
/// `SEQUENCE OF', and a `SEQUENCE SIZE (1..99) OF'.  Without one, every
/// assignment sits one level in.  With a plain one, every *following*
/// assignment jumps several levels deeper; with the SIZE constraint, deeper
/// still.  The token that opens the construct is never closed as far as the
/// grammar is concerned, so the depth it adds is inherited by everything
/// after it, and `END' is the only thing that comes back.
///
/// This is the package's own behaviour, not a difference between the two
/// editors, and it is asserted rather than avoided: it is also the sharpest
/// probe of the tokenizer and SMIE rules in the file, since the exact column
/// each line lands on is a function of `asn1-mode-forward-token',
/// `scan-sexps' over the mode's syntax table, and `smie-rule-parent'.
#[test]
fn a_sequence_of_assignment_leaves_everything_after_it_one_level_deeper() {
    let elisp_form = r##"
(cl-flet ((indented (name middle-line)
            (let ((buffer
                   (asn1-test-visit
                    name
                    (concat "Bestellung DEFINITIONS AUTOMATIC TAGS ::=\n"
                            "BEGIN\n"
                            "Auftrag ::= SEQUENCE {\n"
                            "nummer INTEGER,\n"
                            "kunde Kunde\n"
                            "}\n"
                            middle-line
                            "Zustand ::= ENUMERATED {\n"
                            "offen (0),\n"
                            "versandt (1)\n"
                            "}\n"
                            "END\n"))))
              (indent-region (point-min) (point-max))
              (prog1 (asn1-test-text)
                (set-buffer-modified-p nil)
                (kill-buffer buffer)))))
  (list :without-sequence-of (indented "spec/a.asn1" "")
        :with-sequence-of (indented "spec/b.asn1" "Postenliste ::= SEQUENCE OF Posten\n")
        :with-size-constraint
        (indented "spec/c.asn1" "Postenliste ::= SEQUENCE SIZE (1..99) OF Posten\n")))
"##;

    let expect = expect![[
        r#"OK (:without-sequence-of "Bestellung DEFINITIONS AUTOMATIC TAGS ::=\n\11BEGIN\n\11Auftrag ::= SEQUENCE {\n\11\11\11\11\11\11 nummer INTEGER,\n\11kunde Kunde\n\11}\n\11Zustand ::= ENUMERATED {\n\11\11\11\11\11\11   offen (0),\n\11versandt (1)\n\11}\n\11END\n" :with-sequence-of "Bestellung DEFINITIONS AUTOMATIC TAGS ::=\n\11BEGIN\n\11Auftrag ::= SEQUENCE {\n\11\11\11\11\11\11 nummer INTEGER,\n\11kunde Kunde\n\11}\n\11Postenliste ::= SEQUENCE OF Posten\n\11\11\11\11\11\11\11\11Zustand ::= ENUMERATED {\n\11\11\11\11\11\11\11\11\11\11\11\11\11   offen (0),\n\11\11\11\11\11\11\11\11versandt (1)\n\11\11\11\11\11\11\11\11}\n\11END\n" :with-size-constraint "Bestellung DEFINITIONS AUTOMATIC TAGS ::=\n\11BEGIN\n\11Auftrag ::= SEQUENCE {\n\11\11\11\11\11\11 nummer INTEGER,\n\11kunde Kunde\n\11}\n\11Postenliste ::= SEQUENCE SIZE (1..99) OF Posten\n\11\11\11\11\11\11\11\11\11\11\11 Zustand ::= ENUMERATED {\n\11\11\11\11\11\11\11\11\11\11\11\11\11\11\11\11\11offen (0),\n\11\11\11\11\11\11\11\11\11\11\11 versandt (1)\n\11\11\11\11\11\11\11\11\11\11\11 }\n\11END\n")"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

/// What the module looks like on screen.
///
/// The runs are read back one property at a time and reported as (TEXT FACE)
/// pairs, so the assertion says which characters carry which face rather than
/// quoting a fontified string, whose printed form would also encode the
/// property order.
///
/// The fixture is chosen so that several rules have to disagree correctly.
/// `(0)' `(1)' `(2)' are constants but `(1..65535)' and `(1..99)' are not,
/// because the rule matches digits only.  The names on the left of `::=' are
/// variable names, but `Kunde' and `Posten' used as types are not.  The
/// string literal is not ASCII, and it is faced as one run including both
/// quotes, which is the syntax table's doing rather than a keyword rule.
#[test]
fn fontifying_a_module_marks_keywords_assignment_names_constants_and_strings() {
    let elisp_form = r##"
(let ((buffer (asn1-test-visit "spec/bestellung.asn1" asn1-test-module)))
  (asn1-test-faces))
"##;

    let expect = expect![[
        r#"OK (("-- " font-lock-comment-delimiter-face) ("1 Bestellsystem\n" font-lock-comment-face) ("-- " font-lock-comment-delimiter-face) ("1.1 Grundtypen\n" font-lock-comment-face) ("Bestellung" font-lock-variable-name-face) ("DEFINITIONS" font-lock-keyword-face) ("AUTOMATIC" font-lock-keyword-face) ("TAGS" font-lock-keyword-face) ("BEGIN" font-lock-keyword-face) ("IMPORTS" font-lock-keyword-face) ("FROM" font-lock-keyword-face) ("Auftrag" font-lock-variable-name-face) ("SEQUENCE" font-lock-keyword-face) ("INTEGER" font-lock-keyword-face) ("UTF8String" font-lock-keyword-face) ("OPTIONAL" font-lock-keyword-face) ("Postenliste" font-lock-variable-name-face) ("SEQUENCE" font-lock-keyword-face) ("SIZE" font-lock-keyword-face) ("OF" font-lock-keyword-face) ("-- " font-lock-comment-delimiter-face) ("1.2 Zustände\n" font-lock-comment-face) ("Zustand" font-lock-variable-name-face) ("ENUMERATED" font-lock-keyword-face) ("(0)" font-lock-constant-face) ("(1)" font-lock-constant-face) ("(2)" font-lock-constant-face) ("-- " font-lock-comment-delimiter-face) ("1.2.1 Zahlungsarten\n" font-lock-comment-face) ("Zahlung" font-lock-variable-name-face) ("CHOICE" font-lock-keyword-face) ("standardHinweis" font-lock-variable-name-face) ("UTF8String" font-lock-keyword-face) ("\"Grüße aus München\"" font-lock-string-face) ("END" font-lock-keyword-face))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

/// ASN.1 has two comment forms and the mode implements both through the
/// syntax table rather than through font-lock, so both the parser and the
/// display agree about them.
///
/// The interesting one is `--', which opens a comment and is also closed by a
/// second `--' on the same line, not only by the newline.  The fixture puts a
/// real constraint after such a comment: `Preis ::= INTEGER -- in Cent --
/// (0..100000)'.  The text after the closing `--' is code again, and both
/// `syntax-ppss' and font-lock are asserted to agree about that -- a mode
/// that treated `--' as a line comment would swallow the constraint, and the
/// two assertions would fail together.
///
/// `comment-region' and `uncomment-region' then round-trip a line, which is
/// what a user actually does with the settings the mode installs.
#[test]
fn inline_double_dash_and_block_comments_are_parsed_and_round_trip() {
    let elisp_form = r##"
(let* ((source
        (concat
         "Kommentare DEFINITIONS ::=\n"
         "BEGIN\n"
         "Preis ::= INTEGER -- in Cent -- (0..100000)\n"
         "/* Blockkommentar über\n"
         "   mehrere Zeilen */\n"
         "Waehrung ::= UTF8String\n"
         "END\n"))
       (buffer (asn1-test-visit "spec/kommentare.asn1" source)))
  (list
   :faces (asn1-test-faces)
   :inside-a-comment
   (mapcar (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (search-forward needle)
               (list needle (and (nth 4 (syntax-ppss (match-beginning 0))) t))))
           '("in Cent" "(0..100000)" "Blockkommentar" "mehrere Zeilen" "Waehrung"))
   :commented
   (progn (goto-char (point-min))
          (forward-line 5)
          (comment-region (line-beginning-position) (line-end-position))
          (asn1-test-text))
   :uncommented
   (progn (goto-char (point-min))
          (forward-line 5)
          (uncomment-region (line-beginning-position) (line-end-position))
          (asn1-test-text))))
"##;

    let expect = expect![[
        r#"OK (:faces (("Kommentare" font-lock-variable-name-face) ("DEFINITIONS" font-lock-keyword-face) ("BEGIN" font-lock-keyword-face) ("Preis" font-lock-variable-name-face) ("INTEGER" font-lock-keyword-face) ("-- " font-lock-comment-delimiter-face) ("in Cent --" font-lock-comment-face) ("/* Blockkommentar über\n   mehrere Zeilen */" font-lock-comment-face) ("Waehrung" font-lock-variable-name-face) ("UTF8String" font-lock-keyword-face) ("END" font-lock-keyword-face)) :inside-a-comment (("in Cent" t) ("(0..100000)" nil) ("Blockkommentar" t) ("mehrere Zeilen" t) ("Waehrung" nil)) :commented "Kommentare DEFINITIONS ::=\nBEGIN\nPreis ::= INTEGER -- in Cent -- (0..100000)\n/* Blockkommentar über\n   mehrere Zeilen */\n-- Waehrung ::= UTF8String\nEND\n" :uncommented "Kommentare DEFINITIONS ::=\nBEGIN\nPreis ::= INTEGER -- in Cent -- (0..100000)\n/* Blockkommentar über\n   mehrere Zeilen */\nWaehrung ::= UTF8String\nEND\n")"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

/// Reading a specification rather than editing it: collapse the module to its
/// numbered sections, then jump to a definition by name.
///
/// The mode supplies both halves.  `outline-regexp' matches the `-- 1.2'
/// style section comments and `outline-level' counts the dots, so a `1.2.1'
/// heading is a level below a `1.2' one; the fixture has all three depths so
/// that collapsing to level 2 has something to leave out.  `imenu-generic-
/// expression' picks up every `::=' assignment, including the module header
/// itself, in source order.
///
/// Expanding again is asserted to restore the buffer exactly, not just to
/// make the text visible: outline hides with overlays, and a mode that left
/// one behind would still look right in a diff of the buffer string.
#[test]
fn the_numbered_sections_collapse_and_every_assignment_is_indexed() {
    let elisp_form = r##"
(let ((buffer (asn1-test-visit "spec/bestellung.asn1" asn1-test-module)))
  (require 'imenu)
  (outline-minor-mode 1)
  (list
   :headings (asn1-test-headings)
   :collapsed-to-level-1 (progn (outline-hide-sublevels 1) (asn1-test-visible-text))
   :collapsed-to-level-2 (progn (outline-hide-sublevels 2) (asn1-test-visible-text))
   :restored-exactly (progn (outline-show-all)
                            (equal (asn1-test-visible-text) asn1-test-module))
   :index (mapcar (lambda (entry)
                    (list (car entry)
                          (if (markerp (cdr entry))
                              (marker-position (cdr entry))
                            (cdr entry))))
                  (imenu--make-index-alist t))
   :jumped-to-zahlung (progn (goto-char (point-min))
                             (imenu "Zahlung")
                             (asn1-test-line))))
"##;

    let expect = expect![[
        r#"OK (:headings (("-- 1 Bestellsystem" 1) ("-- 1.1 Grundtypen" 2) ("-- 1.2 Zustände" 2) ("-- 1.2.1 Zahlungsarten" 3)) :collapsed-to-level-1 "-- 1 Bestellsystem\n" :collapsed-to-level-2 "-- 1 Bestellsystem\n-- 1.1 Grundtypen\n-- 1.2 Zustände\n" :restored-exactly t :index (("*Rescan*" -99) ("Bestellung" 38) ("Auftrag" 132) ("Postenliste" 245) ("Zustand" 309) ("Zahlung" 398) ("standardHinweis" 464)) :jumped-to-zahlung (22 0 "Zahlung ::= CHOICE {"))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

/// Typing a definition with the abbreviations the mode builds for itself.
///
/// `asn1-mode-common-setup' derives an abbrev table from the keyword list at
/// mode entry: initials of the words in a keyword, extended a letter at a
/// time until it is unique, shortest keywords first.  The keys are pressed
/// through `execute-kbd-macro' so the expansion happens the way it does for a
/// user -- on the self-inserting space after the abbreviation, not through a
/// direct call to `expand-abbrev'.
///
/// The line is written entirely in the language the mode is for: `se' and
/// `o' expand, while `INTEGER' typed in full stays as it was, which is what
/// separates a working abbrev table from one that rewrites everything.
#[test]
fn typing_keyword_abbreviations_expands_them_into_the_definition() {
    let elisp_form = r##"
(let ((buffer (asn1-test-visit "spec/neu.asn1" "")))
  (abbrev-mode 1)
  (execute-kbd-macro (kbd "N e u e L i s t e SPC : : = SPC s e SPC o SPC I N T E G E R"))
  (list :text (asn1-test-text)
        :abbrev-mode abbrev-mode
        :table-is-the-modes (eq local-abbrev-table asn1-mode-abbrev-table)
        :expansions
        (mapcar (lambda (abbrev)
                  (list abbrev (abbrev-expansion abbrev asn1-mode-abbrev-table)))
                '("se" "s" "o" "be" "e" "op" "utf" "oid" "enu"))))
"##;

    let expect = expect![[
        r#"OK (:text "NeueListe ::= SEQUENCE OF INTEGER" :abbrev-mode t :table-is-the-modes t :expansions (("se" "SEQUENCE") ("s" "SET") ("o" "OF") ("be" "BEGIN") ("e" "END") ("op" "OPTIONAL") ("utf" "UTF8String") ("oid" "OBJECT IDENTIFIER") ("enu" "ENUMERATED")))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

/// The other half of the package: a `.gdmo' file gets `gdmo-mode', which
/// shares the syntax table, the abbrev table and the tokenizer but has its
/// own font-lock rules for X.722 templates.
///
/// The faces show the difference and also show its limit.  `kunde' and
/// `kundenPaket' are function names because they are followed by a template
/// keyword, which is a rule asn1-mode does not have.  But `MANAGED',
/// `DERIVED', `CHARACTERIZED', `REGISTERED', `BEHAVIOUR' and `ATTRIBUTES' are
/// not fontified at all, while `OBJECT', `CLASS', `FROM' and `BY' are:
/// `gdmo-mode-font-lock-keywords' splices in `asn1-mode-font-lock-keywords',
/// which carries `asn1-mode-keywords-regexp', so `gdmo-mode-keywords-regexp'
/// is built at load time and then never used by anything.  Only the GDMO
/// words that happen to also be ASN.1 keywords are highlighted.
#[test]
fn visiting_a_gdmo_file_selects_gdmo_mode_with_its_own_template_names() {
    let elisp_form = r##"
(let ((buffer (asn1-test-visit "spec/kunde.gdmo" asn1-test-gdmo)))
  (list :mode major-mode
        :mode-name mode-name
        :parent (get major-mode 'derived-mode-parent)
        :tab-width tab-width
        :shares-the-asn1-abbrev-table (eq local-abbrev-table asn1-mode-abbrev-table)
        :faces (asn1-test-faces)
        :indented (progn (indent-region (point-min) (point-max))
                         (asn1-test-text))))
"##;

    let expect = expect![[
        r#"OK (:mode gdmo-mode :mode-name "GDMO" :parent prog-mode :tab-width 4 :shares-the-asn1-abbrev-table t :faces (("-- " font-lock-comment-delimiter-face) ("2 Verwaltete Objekte\n" font-lock-comment-face) ("kunde" font-lock-function-name-face) ("OBJECT" font-lock-keyword-face) ("CLASS" font-lock-keyword-face) ("FROM" font-lock-keyword-face) ("\"Rec. X.721 | ISO/IEC 10165-2 : 1992\"" font-lock-string-face) ("BY" font-lock-keyword-face) ("(9)" font-lock-constant-face) ("(3)" font-lock-constant-face) ("(2)" font-lock-constant-face) ("(3)" font-lock-constant-face) ("kundenPaket" font-lock-function-name-face) ("(9)" font-lock-constant-face) ("(3)" font-lock-constant-face) ("(2)" font-lock-constant-face) ("(4)" font-lock-constant-face)) :indented "-- 2 Verwaltete Objekte\nkunde MANAGED OBJECT CLASS\nDERIVED FROM \"Rec. X.721 | ISO/IEC 10165-2 : 1992\":top;\nCHARACTERIZED BY kundenPaket;\nREGISTERED AS { joint-iso-itu-t ms(9) smi(3) part2(2) managedObjectClass(3) 1 };\nkundenPaket PACKAGE\nBEHAVIOUR kundenVerhalten;\nATTRIBUTES kundenNummer GET,\nkundenName GET-REPLACE;\nREGISTERED AS { joint-iso-itu-t ms(9) smi(3) part2(2) package(4) 1 };\n")"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

use expect_test::expect;

use super::{assert_asn1_mode_autoload_parity, assert_asn1_mode_parity};

#[test]
fn descriptor_records_exact_pin_dependency_and_installed_payload() {
    let elisp_form = r##"(let* ((desc (cadr (assq 'asn1-mode package-alist)))
              (dir (package-desc-dir desc)))
          (list
           (package-version-join (package-desc-version desc))
           (package-desc-reqs desc)
           (package-desc-kind desc)
           (sort
            (mapcar #'file-name-nondirectory
                    (directory-files dir t "^[^.].*"))
            #'string<)))"##;
    let expect = expect![[
        r#"OK ("20170729.226" ((emacs (24 3)) (s (1 10 0))) nil ("README-elpa" "asn1-mode-autoloads.el" "asn1-mode-pkg.el" "asn1-mode.el" "asn1-mode.elc"))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn installed_source_has_exact_hash_feature_and_dependency_location() {
    let elisp_form = r##"(let ((source (locate-library "asn1-mode"))
              (dependency (locate-library "s")))
          (list
           (file-name-nondirectory source)
           (with-temp-buffer
             (set-buffer-multibyte nil)
             (insert-file-contents-literally source)
             (secure-hash 'sha256 (current-buffer)))
           (featurep 'asn1-mode)
           (featurep 's)
           (file-name-nondirectory dependency)))"##;
    let expect = expect![[
        r#"OK ("asn1-mode.el" "6fb5ec2f6ddbfcfe253805db663760addad52037621d752596578db05d94bf3c" t t "s.el")"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn complete_declared_callable_surface_has_exact_arities_and_command_status() {
    let elisp_form = r##"(mapcar
          (lambda (symbol)
            (list symbol
                  (help-function-arglist symbol t)
                  (commandp symbol)
                  (macrop symbol)))
          '(asn1-mode-abbrev-table asn1-mode-outline-level
            asn1-mode-regexp-opt asn1-mode-token-match-group
            asn1-mode-forward-token asn1-mode-backward-token
            asn1-mode-debug asn1-mode-backward-token-to
            asn1-mode-smie-rules asn1-mode-toggle-debug
            asn1-mode-common-setup asn1-mode gdmo-mode))"##;
    let expect = expect![
        "OK ((asn1-mode-abbrev-table nil nil nil) (asn1-mode-outline-level nil nil nil) (asn1-mode-regexp-opt (&rest list) nil nil) (asn1-mode-token-match-group (match-data regexp-alist) nil nil) (asn1-mode-forward-token nil nil nil) (asn1-mode-backward-token nil nil nil) (asn1-mode-debug (&rest message) nil nil) (asn1-mode-backward-token-to (token) nil nil) (asn1-mode-smie-rules (kind token) nil nil) (asn1-mode-toggle-debug nil t nil) (asn1-mode-common-setup nil nil nil) (asn1-mode nil t nil) (gdmo-mode nil t nil))"
    ];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn complete_declared_variable_surface_is_bound_with_stable_kinds_and_docs() {
    let elisp_form = r##"(mapcar
          (lambda (symbol)
            (list symbol
                  (boundp symbol)
                  (cond
                   ((keymapp (symbol-value symbol)) 'keymap)
                   ((syntax-table-p (symbol-value symbol)) 'syntax-table)
                   ((stringp (symbol-value symbol)) 'string)
                   ((listp (symbol-value symbol)) 'list)
                   (t (type-of (symbol-value symbol))))
                  (documentation-property symbol
                                          'variable-documentation)))
          '(asn1-mode-map asn1-mode-debug asn1-mode-syntax-table
            asn1-mode-keywords gdmo-mode-keywords
            asn1-mode-keywords-regexp gdmo-mode-keywords-regexp
            asn1-mode-font-lock-keywords gdmo-mode-font-lock-keywords
            asn1-mode-abbrev-table asn1-mode-imenu-expression
            asn1-mode-outline-regexp asn1-mode-token-alist
            gdmo-mode-token-alist asn1-mode-token-alist-2
            gdmo-mode-token-alist-2 asn1-mode-token-regexp
            asn1-mode-token-regexp-2 gdmo-mode-token-regexp
            gdmo-mode-token-regexp-2 asn1-mode-smie-grammar))"##;
    let expect = expect![[
        r#"OK ((asn1-mode-map t keymap "Keymap for ‘asn1-mode’.") (asn1-mode-debug t list nil) (asn1-mode-syntax-table t syntax-table nil) (asn1-mode-keywords t list nil) (gdmo-mode-keywords t list nil) (asn1-mode-keywords-regexp t string "Regexp to match ASN.1 reserved keywords against token.") (gdmo-mode-keywords-regexp t string "Regexp to match GDMO reserved keywords against token.") (asn1-mode-font-lock-keywords t list nil) (gdmo-mode-font-lock-keywords t list nil) (asn1-mode-abbrev-table t obarray nil) (asn1-mode-imenu-expression t list nil) (asn1-mode-outline-regexp t string nil) (asn1-mode-token-alist t list nil) (gdmo-mode-token-alist t list nil) (asn1-mode-token-alist-2 t list nil) (gdmo-mode-token-alist-2 t list nil) (asn1-mode-token-regexp t string nil) (asn1-mode-token-regexp-2 t string nil) (gdmo-mode-token-regexp t string nil) (gdmo-mode-token-regexp-2 t string nil) (asn1-mode-smie-grammar t list nil))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn keyword_catalogs_cover_real_asn1_and_gdmo_language_vocabulary() {
    let elisp_form = r##"(list
          (length asn1-mode-keywords)
          (length gdmo-mode-keywords)
          (seq-take asn1-mode-keywords 12)
          (seq-take (reverse asn1-mode-keywords) 12)
          (mapcar
           (lambda (word)
             (list word
                   (member word asn1-mode-keywords)
                   (member word gdmo-mode-keywords)))
           '("BEGIN" "OBJECT IDENTIFIER" "WITH SYNTAX"
             "MANAGED OBJECT CLASS" "REGISTERED AS"
             "DELETES-CONTAINED-OBJECTS"))
          (length (delete-dups (copy-sequence gdmo-mode-keywords))))"##;
    let expect = expect![[
        r#"OK (94 158 ("ABSENT" "ABSTRACT-SYNTAX" "ALL" "APPLICATION" "AUTOMATIC" "BEGIN" "BIT" "BMPString" "BOOLEAN" "BY" "CHARACTER" "CHOICE") ("WITH SYNTAX" "IDENTIFIED BY" "OBJECT IDENTIFIER" "WITH" "VisibleString" "VideotexString" "UniversalString" "UTF8String" "UTCTime" "UNIVERSAL" "UNIQUE" "UNION") (("BEGIN" ("BEGIN" "BIT" "BMPString" "BOOLEAN" "BY" "CHARACTER" "CHOICE" "CLASS" "COMPONENT" "COMPONENTS" "CONSTRAINED" "CONTAINING" "DATE" "DATE-TIME" "DEFAULT" "DEFINITIONS" "DURATION" "EMBEDDED" "ENCODED" "ENCODING-CONTROL" "END" "ENUMERATED" "EXCEPT" "EXPLICIT" "EXPORTS" "EXTENSIBILITY" "EXTERNAL" "FALSE" "FROM" "GeneralString" "GeneralizedTime" "GraphicString" "IA5String" "IDENTIFIER" "IMPLICIT" "IMPLIED" "IMPORTS" "INCLUDES" "INSTANCE" "INSTRUCTIONS" "INTEGER" "INTERSECTION" "ISO646String" "MAX" "MIN" "MINUS-INFINITY" "NOT-A-NUMBER" "NULL" "NumericString" "OBJECT" "OCTET" "OF" "OID-IRI" "OPTIONAL" "ObjectDescriptor" "PATTERN" "PDV" "PLUS-INFINITY" "PRESENT" "PRIVATE" "PrintableString" "REAL" "RELATIVE-OID" "RELATIVE-OID-IRI" "SEQUENCE" "SET" "SETTINGS" "SIZE" "STRING" "SYNTAX" "T61String" "TAGS" "TIME" "TIME-OF-DAY" "TRUE" "TYPE-IDENTIFIER" "TeletexString" "UNION" "UNIQUE" "UNIVERSAL" "UTCTime" "UTF8String" "UniversalString" "VideotexString" "VisibleString" "WITH" . #1=("OBJECT IDENTIFIER" "IDENTIFIED BY" . #3=("WITH SYNTAX"))) ("BEGIN" "BIT" "BMPString" "BOOLEAN" "BY" "CHARACTER" "CHOICE" "CLASS" "COMPONENT" "COMPONENTS" "CONSTRAINED" "CONTAINING" "DATE" "DATE-TIME" "DEFAULT" "DEFINITIONS" "DURATION" "EMBEDDED" "ENCODED" "ENCODING-CONTROL" "END" "ENUMERATED" "EXCEPT" "EXPLICIT" "EXPORTS" "EXTENSIBILITY" "EXTERNAL" "FALSE" "FROM" "GeneralString" "GeneralizedTime" "GraphicString" "IA5String" "IDENTIFIER" "IMPLICIT" "IMPLIED" "IMPORTS" "INCLUDES" "INSTANCE" "INSTRUCTIONS" "INTEGER" "INTERSECTION" "ISO646String" "MAX" "MIN" "MINUS-INFINITY" "NOT-A-NUMBER" "NULL" "NumericString" "OBJECT" "OCTET" "OF" "OID-IRI" "OPTIONAL" "ObjectDescriptor" "PATTERN" "PDV" "PLUS-INFINITY" "PRESENT" "PRIVATE" "PrintableString" "REAL" "RELATIVE-OID" "RELATIVE-OID-IRI" "SEQUENCE" "SET" "SETTINGS" "SIZE" "STRING" "SYNTAX" "T61String" "TAGS" "TIME" "TIME-OF-DAY" "TRUE" "TYPE-IDENTIFIER" "TeletexString" "UNION" "UNIQUE" "UNIVERSAL" "UTCTime" "UTF8String" "UniversalString" "VideotexString" "VisibleString" "WITH" . #2=("OBJECT IDENTIFIER" "IDENTIFIED BY" . #4=("WITH SYNTAX" . #5=("MANAGED OBJECT CLASS" "DERIVED FROM" "CHARACTERIZED BY" "CONDITIONAL PACKAGES" "PRESENT IF" . #6=("REGISTERED AS" "PACKAGE" "BEHAVIOUR" "ATTRIBUTES" "ATTRIBUTE GROUPS" "ACTIONS" "NOTIFICATIONS" "SET-BY-CREATE" "REPLACE-WITH-DEFAULT" "VALUE" "INITIAL" "PERMITTED" "REQUIRED" "DERIVATION" "RULE" "GET" "REPLACE" "GET-REPLACE" "ADD" "REMOVE" "ADD-REMOVE" "PARAMETER" "CONTEXT" "ACTION-INFO" "ACTION-REPLY" "EVENT-INFO" "EVENT-REPLY" "SPECIFIC-ERROR" "NAME BINDING" "SUBORDINATE OBJECT CLASS" "AND SUBCLASSES" "NAMED BY" "SUPERIOR OBJECT CLASS" "WITH ATTRIBUTE" "CREATE" "DELETE" "WITH-REFERENCE-OBJECT" "WITH-AUTOMATIC-INSTANCE-NAMING" "ONLY-IF-NO-CONTAINED-OBJECTS" . #7=("DELETES-CONTAINED-OBJECTS" "ATTRIBUTE" "WITH ATTRIBUTE SYNTAX" "MATCHES FOR" "PARAMETERS" "EQUALITY" "ORDERING" "SUBSTRINGS" "SET-COMPARISON" "SET-INTERSECTION" "ATTRIBUTE GROUP" "GROUP ELEMENTS" "FIXED" "DESCRIPTION" "DEFINED AS" "ACTION" "MODE CONFIRMED" "WITH INFORMATION SYNTAX" "WITH REPLY SYNTAX" "AND ATTRIBUTE IDS"))))))) ("OBJECT IDENTIFIER" #1# #2#) ("WITH SYNTAX" #3# #4#) ("MANAGED OBJECT CLASS" nil #5#) ("REGISTERED AS" nil #6#) ("DELETES-CONTAINED-OBJECTS" nil #7#)) 158)"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn token_registry_preserves_precedence_and_every_declared_token_family() {
    let elisp_form = r##"(list
          (mapcar #'car asn1-mode-token-alist)
          (mapcar #'car asn1-mode-token-alist-2)
          (mapcar #'car gdmo-mode-token-alist)
          (mapcar #'car gdmo-mode-token-alist-2)
          (secure-hash 'sha256 asn1-mode-token-regexp)
          (secure-hash 'sha256 gdmo-mode-token-regexp)
          (mapcar
           (lambda (token)
             (cons token
                   (cdr (assoc token gdmo-mode-token-alist))))
           '("_TAG_KIND" "_WITH_SYNTAX" "_SET" "_SEQ"
             "_GDMO_OPEN" "_REGISTERED_AS")))"##;
    let expect = expect![[
        r#"OK (("_TAG_KIND" "_WITH_SYNTAX" "_CLASS" "TAGS" "DEFINITIONS" "EXPORTS" "BEGIN" "END" "SIZE" "OF" "IMPORTS" "_SET" "_SEQ" "_UCASE_ID" "_LITERAL" "_XML_OPENER" "_XML_CLOSER" "_XML_SINGLE" "..." "::=") ("FROM" "_LCASE_ID" "_UCASE_ID") ("_TAG_KIND" "_WITH_SYNTAX" "_CLASS" "TAGS" "DEFINITIONS" "EXPORTS" "BEGIN" "END" "SIZE" "OF" "IMPORTS" "_SET" "_SEQ" "_UCASE_ID" "_LITERAL" "_XML_OPENER" "_XML_CLOSER" "_XML_SINGLE" "..." "::=" "_GDMO_OPEN" "_REGISTERED_AS") ("FROM" "_LCASE_ID" "_UCASE_ID" "_GDMO_OPEN") "f874a55c308a0c29ed55db12708907a0189e0492b9ca14f6293ded4b9a345058" "3f84e4f5cd6c5e5fb7aaca222c368e36cf5f74a7727ce4a222c12691cb30db36" (("_TAG_KIND" . "\\b\\(AUTOMATIC\\|\\(?:EX\\|IM\\)PLICIT\\)\\b") ("_WITH_SYNTAX" . "\\b\\(WITH SYNTAX\\)\\b") ("_SET" . "\\b\\(SE\\(?:\\(?:QUENCE\\|T\\) OF\\)\\)\\b") ("_SEQ" . "\\b\\(CHOICE\\|ENUMERATED\\|SEQUENCE\\)\\b") ("_GDMO_OPEN" . "\\b\\(A\\(?:CTIONS?\\|TTRIBUTE\\(?: GROUPS?\\|S\\)\\)\\|BEHAVIOUR\\|C\\(?:HARACTERIZED BY\\|ON\\(?:DITIONAL PACKAGES\\|TEXT\\)\\|REATE\\)\\|DE\\(?:LETE\\|RIVED FROM\\|SCRIPTION\\)\\|FIXED\\|GROUP ELEMENTS\\|M\\(?:ANAGED OBJECT CLASS\\|ODE CONFIRMED\\)\\|N\\(?:AME\\(?: BINDING\\|D BY SUPERIOR OBJECT CLASS\\)\\|OTIFICATIONS?\\)\\|PA\\(?:CKAGE\\|RAMETERS?\\)\\|SUBORDINATE OBJECT CLASS\\|WITH \\(?:ATTRIBUTE\\|\\(?:INFORMATION\\|REPLY\\) SYNTAX\\)\\)\\b") ("_REGISTERED_AS" . "\\b\\(REGISTERED AS\\)\\b")))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn modes_and_file_associations_publish_the_documented_editing_contract() {
    let elisp_form = r##"(list
          (cdr (assoc "\\.asn1$" auto-mode-alist))
          (cdr (assoc "\\.gdmo$" auto-mode-alist))
          (mapcar
           (lambda (mode)
             (list mode
                   (get mode 'derived-mode-parent)
                   (documentation mode)
                   (commandp mode)))
           '(asn1-mode gdmo-mode))
          (lookup-key asn1-mode-map [foo]))"##;
    let expect = expect![[
        r#"OK (asn1-mode gdmo-mode ((asn1-mode prog-mode #("Major mode for editing ASN.1 text files in Emacs.\n\n\nKey             Binding\n-------------------------------------------------------------------------------\n<foo>\11\11asn1-mode-do-foo\n\nEntry to this mode calls the value of ‘asn1-mode-hook’\nif that value is non-nil." 76 155 (face separator-line) 156 161 (font-lock-face help-key-binding face help-key-binding)) t) (gdmo-mode prog-mode #("Major mode for editing GDMO text files in Emacs.\n\n\nKey             Binding\n-------------------------------------------------------------------------------\n<foo>\11\11asn1-mode-do-foo\n\nEntry to this mode calls the value of ‘asn1-mode-hook’\nif that value is non-nil.\n\nIn addition to any hooks its parent mode ‘prog-mode’ might have run,\nthis mode runs the hook ‘gdmo-mode-hook’, as the final or penultimate\nstep during initialization." 75 154 (face separator-line) 155 160 (font-lock-face help-key-binding face help-key-binding)) t)) asn1-mode-do-foo)"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn autoload_file_exposes_modes_and_associations_without_loading_implementation() {
    let elisp_form = r##"(list
          (featurep 'asn1-mode)
          (mapcar
           (lambda (symbol)
             (list symbol
                   (fboundp symbol)
                   (autoloadp (symbol-function symbol))
                   (commandp symbol)))
           '(asn1-mode gdmo-mode asn1-mode-forward-token))
          (cdr (assoc "\\.asn1$" auto-mode-alist))
          (cdr (assoc "\\.gdmo$" auto-mode-alist)))"##;
    let expect = expect![
        "OK (nil ((asn1-mode t t t) (gdmo-mode t t t) (asn1-mode-forward-token nil nil nil)) asn1-mode gdmo-mode)"
    ];
    assert_asn1_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn declared_symbols_retain_exact_source_ownership() {
    let elisp_form = r##"(mapcar
          (lambda (symbol)
            (list symbol
                  (and (symbol-file symbol 'defun)
                       (file-name-nondirectory
                        (symbol-file symbol 'defun)))
                  (and (symbol-file symbol 'defvar)
                       (file-name-nondirectory
                        (symbol-file symbol 'defvar)))))
          '(asn1-mode asn1-mode-forward-token asn1-mode-smie-rules
            gdmo-mode asn1-mode-debug asn1-mode-keywords
            asn1-mode-token-alist asn1-mode-smie-grammar))"##;
    let expect = expect![[
        r#"OK ((asn1-mode "asn1-mode.el" nil) (asn1-mode-forward-token "asn1-mode.el" nil) (asn1-mode-smie-rules "asn1-mode.el" nil) (gdmo-mode "asn1-mode.el" nil) (asn1-mode-debug "asn1-mode.el" "asn1-mode.el") (asn1-mode-keywords nil "asn1-mode.el") (asn1-mode-token-alist nil "asn1-mode.el") (asn1-mode-smie-grammar nil "asn1-mode.el"))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn repeated_source_loading_is_idempotent_for_features_associations_and_modes() {
    let elisp_form = r##"(let ((source (locate-library "asn1-mode"))
              snapshots)
          (dotimes (_ 3)
            (load source nil 'nomessage)
            (push
             (list
              (cl-count 'asn1-mode features)
              (cl-count '("\\.asn1$" . asn1-mode)
                        auto-mode-alist :test #'equal)
              (cl-count '("\\.gdmo$" . gdmo-mode)
                        auto-mode-alist :test #'equal)
              (help-function-arglist 'asn1-mode-forward-token t))
             snapshots))
          (list
           (cl-count 'asn1-mode features)
           (and (equal (nth 0 snapshots) (nth 1 snapshots))
                (equal (nth 1 snapshots) (nth 2 snapshots)))))"##;
    let expect = expect!["OK (1 t)"];
    assert_asn1_mode_parity(elisp_form, expect);
}

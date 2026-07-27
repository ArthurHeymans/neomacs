use expect_test::expect;

use super::assert_apparmor_mode_parity;

#[test]
fn apparmor_mode_initializes_complete_buffer_local_editing_contract() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (list
          major-mode
          mode-name
          (eq (syntax-table) apparmor-mode-syntax-table)
          font-lock-defaults
          font-lock-multiline
          (eq syntax-propertize-function
              apparmor-mode--syntax-propertize-function)
          indent-tabs-mode
          tab-width
          indent-line-function
          (memq #'apparmor-mode-completion-at-point
                completion-at-point-functions)
          imenu-generic-expression
          comment-start
          comment-end
          (memq #'apparmor-mode-flymake
                flymake-diagnostic-functions)))"##;
    let expect = expect![[
        r##"OK (apparmor-mode "AppArmor" t ((("\\_<\\(a\\(?:ll\\|udit\\)\\|c\\(?:apability\\|hmod\\)\\|d\\(?:bus\\|e\\(?:legate\\|ny\\)\\)\\|f\\(?:ile\\|lags\\)\\|i\\(?:nclude\\(?: if exists\\)?\\|o_uring\\)\\|link\\|m\\(?:ount\\|queue\\)\\|network\\|o\\(?:n\\|wner\\)\\|p\\(?:ivot_root\\|rofile\\)\\|quiet\\|r\\(?:\\(?:emoun\\|limi\\)t\\)\\|s\\(?:afe\\|ubset\\)\\|to\\|u\\(?:mount\\|nsafe\\|serns\\)\\)\\_>" . font-lock-keyword-face) ("\\_<\\(as\\|c\\(?:ore\\|pu\\)\\|data\\|fsize\\|locks\\|m\\(?:emlock\\|sgqueue\\)\\|n\\(?:ice\\|ofile\\|proc\\)\\|r\\(?:ss\\|tprio\\)\\|s\\(?:igpending\\|tack\\)\\)\\_>" . font-lock-type-face) (",\\s-*$" quote font-lock-builtin-face) ("->" quote font-lock-builtin-face) ("[=\\+()]" quote font-lock-builtin-face) ("+=" quote font-lock-builtin-face) ("<=" quote font-lock-builtin-face) ("^\\s-*\\(#?abi\\)\\s-+\\([<\"][[:graph:]]+[\">]\\)" (1 font-lock-preprocessor-face t) (2 font-lock-string-face t)) ("^\\s-*\\(#?include\\( if exists\\)?\\)\\s-+\\([<\"][[:graph:]]+[\">]\\)" (1 font-lock-preprocessor-face t) (3 font-lock-string-face t)) (apparmor-mode--variable-name-matcher 0 font-lock-variable-name-face t) ("^\\s-*\\(@{[[:alpha:]][[:alnum:]_]*}\\)\\s-*\\(\\+?=\\)\\s-*\\([[:graph:]]+\\)\\(\\s-+\\([[:graph:]]+\\)\\)?\\s-*\\(#.*\\)?$" 1 font-lock-variable-name-face t) ("^\\s-*\\(@{[[:alpha:]][[:alnum:]_]*}\\)\\s-*\\(\\+?=\\)\\s-*\\([[:graph:]]+\\)\\(\\s-+\\([[:graph:]]+\\)\\)?\\s-*\\(#.*\\)?$" 2 font-lock-builtin-face t) ("^\\s-*\\(\\(profile\\)\\s-+\\(\\([[:alnum:]]+\\)\\s-+\\)?\\)?\\(\\^?\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\(\\s-+\\(flags\\)=(\\(\\(?:attach_disconnected\\|c\\(?:hroot_\\(?:attach\\|no_attach\\|relative\\)\\|omplain\\)\\|debug\\|enforce\\|kill\\|n\\(?:amespace_relative\\|o_attach_disconnected\\)\\|unconfined\\)\\s-*\\)*)\\)?\\s-+{\\s-*$" (4 font-lock-function-name-face t nil) (5 font-lock-variable-name-face t) (apparmor-mode--glob-path-matcher (progn (goto-char (match-beginning 5)) (match-end 5)) nil (0 font-lock-builtin-face t t)) (apparmor-mode--glob-wildcard-matcher (progn (goto-char (match-beginning 5)) (match-end 5)) nil (0 'font-lock-regexp-grouping-construct t t))) ("^\\s-*\\(capability\\)\\(\\(?:\\s-+\\(?:audit_\\(?:control\\|write\\)\\|chown\\|dac_\\(?:override\\|read_search\\)\\|f\\(?:owner\\|setid\\)\\|ipc_\\(?:lock\\|owner\\)\\|kill\\|l\\(?:\\(?:eas\\|inux_immutabl\\)e\\)\\|m\\(?:ac_\\(?:admin\\|override\\)\\|knod\\)\\|net_\\(?:admin\\|b\\(?:ind_service\\|roadcast\\)\\|raw\\)\\|s\\(?:et\\(?:fcap\\|gid\\|pcap\\|uid\\)\\|ys\\(?:_\\(?:admin\\|boot\\|chroot\\|module\\|nice\\|p\\(?:acct\\|trace\\)\\|r\\(?:awio\\|esource\\)\\|t\\(?:ime\\|ty_config\\)\\)\\|log\\)\\)\\)\\)+\\)" 2 font-lock-type-face t) ("^\\s-*\\(\\(audit\\|owner\\|deny\\)\\s-+\\)*\\(file\\s-+\\)?\\([CPUaciklmpruwx]+\\)\\s-+\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\s-*\\(->\\s-+\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\)?\\s-*," (3 font-lock-keyword-face nil t) (4 font-lock-constant-face t) (7 font-lock-function-name-face nil t) (apparmor-mode--glob-path-matcher (progn (goto-char (match-beginning 5)) (match-end 5)) nil (0 font-lock-builtin-face t t)) (apparmor-mode--glob-wildcard-matcher (progn (goto-char (match-beginning 5)) (match-end 5)) nil (0 'font-lock-regexp-grouping-construct t t))) ("^\\s-*\\(\\(audit\\|owner\\|deny\\)\\s-+\\)*\\(file\\s-+\\)?\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\s-+\\([CPUaciklmpruwx]+\\)\\s-*\\(->\\s-+\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\)?\\s-*," (3 font-lock-keyword-face nil t) (5 font-lock-constant-face t) (7 font-lock-function-name-face nil t) (apparmor-mode--glob-path-matcher (progn (goto-char (match-beginning 4)) (match-end 4)) nil (0 font-lock-builtin-face t t)) (apparmor-mode--glob-wildcard-matcher (progn (goto-char (match-beginning 4)) (match-end 4)) nil (0 'font-lock-regexp-grouping-construct t t))) ("^\\s-*\\(\\(audit\\|quiet\\|deny\\)\\s-+\\)*network\\s-*\\(\\<\\(accept\\|bind\\|c\\(?:onnect\\|reate\\)\\|fcntl\\|get\\(?:peer\\(?:name\\|sec\\)\\|sock\\(?:name\\|opt\\)\\)\\|ioctl\\|listen\\|override_creds\\|re\\(?:ad\\|ceive\\)\\|s\\(?:e\\(?:nd\\|tsockopt\\)\\|hutdown\\|qpoll\\)\\|write\\)\\>\\)?\\s-*\\(\\<\\(a\\(?:ppletalk\\|sh\\|tm\\(?:[ps]vc\\)\\|x25\\)\\|b\\(?:luetooth\\|ridge\\)\\|econet\\|i\\(?:net6?\\|px\\|rda\\)\\|key\\|net\\(?:beui\\|rom\\)\\|p\\(?:acket\\|ppox\\)\\|rose\\|s\\(?:ecurity\\|na\\)\\|unix\\|wanpipe\\|x25\\)\\>\\)?\\s-*\\(\\<\\(d\\(?:ccp\\|gram\\)\\|packet\\|r\\(?:aw\\|dm\\)\\|s\\(?:eqpacket\\|tream\\)\\)\\>\\)?\\s-*\\(\\<\\(\\(?:icm\\|tc\\|ud\\)p\\)\\>\\)?\\s-*\\(delegate to\\s-+\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\)?\\s-*," (3 font-lock-constant-face t t) (4 font-lock-function-name-face t t) (5 font-lock-variable-name-face t t) (6 font-lock-type-face t t)) ("^\\s-*\\(\\(audit\\|deny\\)\\s-+\\)?dbus\\s-*\\(\\(bus\\)=\\(system\\|session\\)\\)?\\s-*\\(\\(dest\\)=\\([[:alpha:].]+\\)\\)?\\s-*\\(\\(path\\)=\\([[:alpha:]/]+\\)\\)?\\s-*\\(\\(interface\\)=\\([[:alpha:].]+\\)\\)?\\s-*\\(\\(method\\)=\\([[:alpha:]_]+\\)\\)?\\s-*\\(\\<\\(acquire\\|bind\\|r\\(?:e\\(?:ad\\|ceive\\)\\|w\\)\\|send\\|write\\|[rw]\\)\\>\\|(\\<\\(acquire\\|bind\\|r\\(?:e\\(?:ad\\|ceive\\)\\|w\\)\\|send\\|write\\|[rw]\\)\\>\\(\\<\\(acquire\\|bind\\|r\\(?:e\\(?:ad\\|ceive\\)\\|w\\)\\|send\\|write\\|[rw]\\)\\>,\\s-+\\)\\)?\\s-*," (4 font-lock-variable-name-face t t) (5 font-lock-constant-face t t) (7 font-lock-variable-name-face t t) (10 font-lock-variable-name-face t t) (13 font-lock-variable-name-face t t) (16 font-lock-variable-name-face t t)))) t t nil 2 apparmor-mode-indent-line (apparmor-mode-completion-at-point tags-completion-at-point-function) (("Profiles" "^\\s-*\\(\\(profile\\)\\s-+\\(\\([[:alnum:]]+\\)\\s-+\\)?\\)?\\(\\^?\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\(\\s-+\\(flags\\)=(\\(\\(?:attach_disconnected\\|c\\(?:hroot_\\(?:attach\\|no_attach\\|relative\\)\\|omplain\\)\\|debug\\|enforce\\|kill\\|n\\(?:amespace_relative\\|o_attach_disconnected\\)\\|unconfined\\)\\s-*\\)*)\\)?\\s-+{\\s-*$" 5)) "#" "" (apparmor-mode-flymake t))"##
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_syntax_table_and_propertizer_parse_policy_tokens() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert
          "# real comment\n"
          "/usr/lib/libfoo.so.1#2 mr,\n"
          "include <abstractions/base>\n"
          "@{HOME}/file rw,\n")
         (syntax-propertize (point-max))
         (let ((chars '(?# ?\n ?< ?> ?@ ?,)))
           (list
            (mapcar
             (lambda (char)
               (list char
                     (char-syntax char)
                     (aref apparmor-mode-syntax-table char)))
             chars)
            (mapcar
             (lambda (target)
               (goto-char (point-min))
               (search-forward target)
               (list target
                     (nth 4
                          (syntax-ppss
                           (match-beginning 0)))))
             '("real comment" "#" "2" "abstractions/base"
               "@{HOME}")))))"##;
    let expect = expect![[
        r##"OK (((35 60 (11)) (10 62 (12)) (60 40 (4 . 62)) (62 41 (5 . 60)) (64 95 (3)) (44 46 (1))) (("real comment" t) ("#" nil) ("2" nil) ("abstractions/base" nil) ("@{HOME}" nil)))"##
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_tracks_nested_block_depth_and_previous_real_line() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert
          "profile outer /usr/bin/outer {\n"
          "  # comment\n\n"
          "  profile inner /usr/bin/inner {\n"
          "    capability net_admin,\n"
          "  }\n"
          "}\n")
         (list
          (mapcar
           (lambda (line)
             (goto-char (point-min))
             (forward-line line)
             (list line (apparmor-mode--block-depth)))
           '(0 1 2 3 4 5 6 7))
          (progn
            (goto-char (point-min))
            (forward-line 3)
            (apparmor-mode--find-first-previous-non-blank-line)
            (list (line-number-at-pos)
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position))))))"##;
    let expect = expect![[
        r#"OK (((0 0) (1 1) (2 1) (3 1) (4 2) (5 2) (6 1) (7 0)) (1 "profile outer /usr/bin/outer {"))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_indent_line_preserves_or_moves_point_by_call_position() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (setq-local apparmor-mode-indent-offset 3)
         (insert "profile demo /usr/bin/demo {\ncapability net_admin,\n}\n")
         (goto-char (point-min))
         (forward-line 1)
         (forward-char 5)
         (let ((inside-before (point)))
           (apparmor-mode-indent-line)
           (let ((inside-after (point))
                 (inside-column (current-column)))
             (goto-char (point-min))
             (forward-line 2)
             (let ((bol-before (point)))
               (apparmor-mode-indent-line)
               (list
                (buffer-string)
                inside-before inside-after inside-column
                bol-before (point) (current-column))))))"##;
    let expect = expect![[
        r#"OK ("profile demo /usr/bin/demo {\ncapability net_admin,\n}\n" 35 35 5 52 52 0)"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

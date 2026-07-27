use expect_test::expect;

use super::assert_apparmor_mode_parity;

#[test]
fn apparmor_mode_descriptor_records_exact_pin_dependency_and_payload() {
    let elisp_form = r##"(let* ((desc
                (cadr (assq 'apparmor-mode package-alist)))
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
        r#"OK ("20260515.454" ((emacs (26 1))) nil ("README-elpa" "apparmor-mode-autoloads.el" "apparmor-mode-pkg.el" "apparmor-mode.el" "apparmor-mode.elc"))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_complete_callable_surface_arities_and_commands_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (stringp (documentation symbol t))))
         '(apparmor-mode-get-apparmor-parser-executable-path
           apparmor-mode--in-comment-p
           apparmor-mode--variable-name-matcher
           apparmor-mode--glob-wildcard-matcher
           apparmor-mode--glob-path-matcher
           apparmor-mode-complete-include
           apparmor-mode-completion-at-point
           apparmor-mode-indent-line
           apparmor-mode--find-first-previous-non-blank-line
           apparmor-mode--block-depth
           apparmor-mode--indent-line
           apparmor-mode
           apparmor-mode-flymake
           apparmor-mode-setup-flymake-backend))"##;
    let expect = expect![
        "OK ((apparmor-mode-get-apparmor-parser-executable-path nil nil t) (apparmor-mode--in-comment-p (pos) nil t) (apparmor-mode--variable-name-matcher (limit) nil t) (apparmor-mode--glob-wildcard-matcher (limit) nil t) (apparmor-mode--glob-path-matcher (limit) nil t) (apparmor-mode-complete-include (prefix &optional local) nil t) (apparmor-mode-completion-at-point nil nil t) (apparmor-mode-indent-line nil t t) (apparmor-mode--find-first-previous-non-blank-line nil nil t) (apparmor-mode--block-depth nil nil t) (apparmor-mode--indent-line nil nil t) (apparmor-mode nil t t) (apparmor-mode-flymake (report-fn &rest _args) nil t) (apparmor-mode-setup-flymake-backend nil nil t))"
    ];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_keyword_capability_and_profile_vocabularies_are_exact() {
    let elisp_form = r##"(list
         apparmor-mode-keywords
         apparmor-mode-profile-flags
         apparmor-mode-capabilities
         (length apparmor-mode-keywords)
         (length apparmor-mode-profile-flags)
         (length apparmor-mode-capabilities)
         (delete-dups (copy-sequence apparmor-mode-keywords)))"##;
    let expect = expect![[
        r#"OK (("all" "audit" "capability" "chmod" "delegate" "dbus" "deny" "file" "flags" "io_uring" "include" "include if exists" "link" "mount" "mqueue" "network" "on" "owner" "pivot_root" "profile" "quiet" "remount" "rlimit" "safe" "subset" "to" "umount" "unsafe" "userns") ("enforce" "complain" "debug" "kill" "chroot_relative" "namespace_relative" "attach_disconnected" "no_attach_disconnected" "chroot_attach" "chroot_no_attach" "unconfined") ("audit_control" "audit_write" "chown" "dac_override" "dac_read_search" "fowner" "fsetid" "ipc_lock" "ipc_owner" "kill" "lease" "linux_immutable" "mac_admin" "mac_override" "mknod" "net_admin" "net_bind_service" "net_broadcast" "net_raw" "setfcap" "setgid" "setpcap" "setuid" "syslog" "sys_admin" "sys_boot" "sys_chroot" "sys_module" "sys_nice" "sys_pacct" "sys_ptrace" "sys_rawio" "sys_resource" "sys_time" "sys_tty_config") 29 11 35 ("all" "audit" "capability" "chmod" "delegate" "dbus" "deny" "file" "flags" "io_uring" "include" "include if exists" "link" "mount" "mqueue" "network" "on" "owner" "pivot_root" "profile" "quiet" "remount" "rlimit" "safe" "subset" "to" "umount" "unsafe" "userns"))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_network_dbus_and_rlimit_vocabularies_are_exact() {
    let elisp_form = r##"(list
         apparmor-mode-network-permissions
         apparmor-mode-network-domains
         apparmor-mode-network-types
         apparmor-mode-network-protocols
         apparmor-mode-dbus-permissions
         apparmor-mode-rlimit-types
         (mapcar
          #'length
          (list apparmor-mode-network-permissions
                apparmor-mode-network-domains
                apparmor-mode-network-types
                apparmor-mode-network-protocols
                apparmor-mode-dbus-permissions
                apparmor-mode-rlimit-types)))"##;
    let expect = expect![[
        r#"OK (("create" "accept" "bind" "connect" "listen" "read" "write" "send" "receive" "getsockname" "getpeername" "getsockopt" "setsockopt" "fcntl" "ioctl" "shutdown" "getpeersec" "sqpoll" "override_creds") ("inet" "ax25" "ipx" "appletalk" "netrom" "bridge" "atmpvc" "x25" "inet6" "rose" "netbeui" "security" "key" "packet" "ash" "econet" "atmsvc" "sna" "irda" "pppox" "wanpipe" "bluetooth" "unix") ("stream" "dgram" "seqpacket" "raw" "rdm" "packet" "dccp") ("tcp" "udp" "icmp") ("r" "w" "rw" "send" "receive" "acquire" "bind" "read" "write") ("fsize" "data" "stack" "core" "rss" "as" "memlock" "msgqueue" "nofile" "locks" "sigpending" "nproc" "rtprio" "cpu" "nice") (19 23 7 3 9 15))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_custom_defaults_and_all_parser_regexps_are_exact() {
    let elisp_form = r##"(list
         apparmor-mode-indent-offset
         apparmor-mode-apparmor-parser-executable
         (custom-variable-p 'apparmor-mode-indent-offset)
         (custom-variable-p
          'apparmor-mode-apparmor-parser-executable)
         apparmor-mode-abi-regexp
         apparmor-mode-include-regexp
         apparmor-mode-capability-regexp
         apparmor-mode-variable-name-regexp
         apparmor-mode-variable-regexp
         apparmor-mode-profile-name-regexp
         apparmor-mode-profile-attachment-regexp
         apparmor-mode-profile-flags-regexp
         apparmor-mode-profile-regexp
         apparmor-mode-file-rule-permissions-regexp
         apparmor-mode-file-rule-permissions-prefix-regexp
         apparmor-mode-file-rule-permissions-suffix-regexp
         apparmor-mode-network-rule-regexp
         apparmor-mode-dbus-rule-regexp
         apparmor-mode-glob-regexp
         apparmor-mode-glob-wildcard-regexp)"##;
    let expect = expect![[
        r#"OK (2 "apparmor_parser" ((funcall #'#[nil (2) #1=(flycheck-checkers t)])) ((funcall #'#[nil ("apparmor_parser") #1#])) "^\\s-*\\(#?abi\\)\\s-+\\([<\"][[:graph:]]+[\">]\\)" "^\\s-*\\(#?include\\( if exists\\)?\\)\\s-+\\([<\"][[:graph:]]+[\">]\\)" "^\\s-*\\(capability\\)\\(\\(?:\\s-+\\(?:audit_\\(?:control\\|write\\)\\|chown\\|dac_\\(?:override\\|read_search\\)\\|f\\(?:owner\\|setid\\)\\|ipc_\\(?:lock\\|owner\\)\\|kill\\|l\\(?:\\(?:eas\\|inux_immutabl\\)e\\)\\|m\\(?:ac_\\(?:admin\\|override\\)\\|knod\\)\\|net_\\(?:admin\\|b\\(?:ind_service\\|roadcast\\)\\|raw\\)\\|s\\(?:et\\(?:fcap\\|gid\\|pcap\\|uid\\)\\|ys\\(?:_\\(?:admin\\|boot\\|chroot\\|module\\|nice\\|p\\(?:acct\\|trace\\)\\|r\\(?:awio\\|esource\\)\\|t\\(?:ime\\|ty_config\\)\\)\\|log\\)\\)\\)\\)+\\)" "@{[[:alpha:]][[:alnum:]_]*}" "^\\s-*\\(@{[[:alpha:]][[:alnum:]_]*}\\)\\s-*\\(\\+?=\\)\\s-*\\([[:graph:]]+\\)\\(\\s-+\\([[:graph:]]+\\)\\)?\\s-*\\(#.*\\)?$" "[[:alnum:]]+" "\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)" "\\(flags\\)=(\\(\\(?:attach_disconnected\\|c\\(?:hroot_\\(?:attach\\|no_attach\\|relative\\)\\|omplain\\)\\|debug\\|enforce\\|kill\\|n\\(?:amespace_relative\\|o_attach_disconnected\\)\\|unconfined\\)\\s-*\\)*)" "^\\s-*\\(\\(profile\\)\\s-+\\(\\([[:alnum:]]+\\)\\s-+\\)?\\)?\\(\\^?\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\(\\s-+\\(flags\\)=(\\(\\(?:attach_disconnected\\|c\\(?:hroot_\\(?:attach\\|no_attach\\|relative\\)\\|omplain\\)\\|debug\\|enforce\\|kill\\|n\\(?:amespace_relative\\|o_attach_disconnected\\)\\|unconfined\\)\\s-*\\)*)\\)?\\s-+{\\s-*$" "[CPUaciklmpruwx]+" "^\\s-*\\(\\(audit\\|owner\\|deny\\)\\s-+\\)*\\(file\\s-+\\)?\\([CPUaciklmpruwx]+\\)\\s-+\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\s-*\\(->\\s-+\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\)?\\s-*," "^\\s-*\\(\\(audit\\|owner\\|deny\\)\\s-+\\)*\\(file\\s-+\\)?\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\s-+\\([CPUaciklmpruwx]+\\)\\s-*\\(->\\s-+\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\)?\\s-*," "^\\s-*\\(\\(audit\\|quiet\\|deny\\)\\s-+\\)*network\\s-*\\(\\<\\(accept\\|bind\\|c\\(?:onnect\\|reate\\)\\|fcntl\\|get\\(?:peer\\(?:name\\|sec\\)\\|sock\\(?:name\\|opt\\)\\)\\|ioctl\\|listen\\|override_creds\\|re\\(?:ad\\|ceive\\)\\|s\\(?:e\\(?:nd\\|tsockopt\\)\\|hutdown\\|qpoll\\)\\|write\\)\\>\\)?\\s-*\\(\\<\\(a\\(?:ppletalk\\|sh\\|tm\\(?:[ps]vc\\)\\|x25\\)\\|b\\(?:luetooth\\|ridge\\)\\|econet\\|i\\(?:net6?\\|px\\|rda\\)\\|key\\|net\\(?:beui\\|rom\\)\\|p\\(?:acket\\|ppox\\)\\|rose\\|s\\(?:ecurity\\|na\\)\\|unix\\|wanpipe\\|x25\\)\\>\\)?\\s-*\\(\\<\\(d\\(?:ccp\\|gram\\)\\|packet\\|r\\(?:aw\\|dm\\)\\|s\\(?:eqpacket\\|tream\\)\\)\\>\\)?\\s-*\\(\\<\\(\\(?:icm\\|tc\\|ud\\)p\\)\\>\\)?\\s-*\\(delegate to\\s-+\\(\\(?:\"[^\"\n]*\"\\|[][[:alnum:]*@/_{},-.?#]+\\)\\)\\)?\\s-*," "^\\s-*\\(\\(audit\\|deny\\)\\s-+\\)?dbus\\s-*\\(\\(bus\\)=\\(system\\|session\\)\\)?\\s-*\\(\\(dest\\)=\\([[:alpha:].]+\\)\\)?\\s-*\\(\\(path\\)=\\([[:alpha:]/]+\\)\\)?\\s-*\\(\\(interface\\)=\\([[:alpha:].]+\\)\\)?\\s-*\\(\\(method\\)=\\([[:alpha:]_]+\\)\\)?\\s-*\\(\\<\\(acquire\\|bind\\|r\\(?:e\\(?:ad\\|ceive\\)\\|w\\)\\|send\\|write\\|[rw]\\)\\>\\|(\\<\\(acquire\\|bind\\|r\\(?:e\\(?:ad\\|ceive\\)\\|w\\)\\|send\\|write\\|[rw]\\)\\>\\(\\<\\(acquire\\|bind\\|r\\(?:e\\(?:ad\\|ceive\\)\\|w\\)\\|send\\|write\\|[rw]\\)\\>,\\s-+\\)\\)?\\s-*," "\\*\\*\\|[*{}?]" "\\*\\*\\|[*?]")"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_autoload_side_effects_register_hooks_and_exact_path_rules() {
    let elisp_form = r##"(list
         (featurep 'apparmor-mode)
         (memq #'apparmor-mode-setup-flymake-backend
               apparmor-mode-hook)
         (cl-remove-if-not
          (lambda (entry)
            (eq (cdr entry) 'apparmor-mode))
          auto-mode-alist)
         (get 'apparmor-mode 'derived-mode-parent)
         (get 'apparmor-mode 'definition-name)
         (get 'apparmor-mode 'mode-class))"##;
    let expect = expect![[
        r#"OK (t (apparmor-mode-setup-flymake-backend) (("\\`/var/lib/snapd/apparmor/profiles/" . apparmor-mode) ("\\`/etc/apparmor\\.d/" . apparmor-mode)) prog-mode nil nil)"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_reload_is_idempotent_for_feature_hook_and_path_rules() {
    let elisp_form = r##"(let ((source
                (locate-library "apparmor-mode")))
         (load source nil 'nomessage)
         (load source nil 'nomessage)
         (list
          (cl-count 'apparmor-mode features)
          (cl-count
           #'apparmor-mode-setup-flymake-backend
           apparmor-mode-hook)
          (mapcar
           (lambda (regexp)
             (cl-count-if
              (lambda (entry)
                (and (equal (car entry) regexp)
                     (eq (cdr entry) 'apparmor-mode)))
              auto-mode-alist))
           '("\\`/etc/apparmor\\.d/"
             "\\`/var/lib/snapd/apparmor/profiles/"))))"##;
    let expect = expect!["OK (1 1 (1 1))"];
    assert_apparmor_mode_parity(elisp_form, expect);
}

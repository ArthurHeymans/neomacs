use expect_test::expect;

use super::assert_apparmor_mode_parity;

#[test]
fn apparmor_mode_indents_multiline_dbus_rule_from_upstream_contract() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert "\ndbus (send)\nbus=session,\n")
         (let ((before (buffer-string)))
           (indent-region (point-min) (point-max))
           (list before (buffer-string)
                 (point) (current-indentation))))"##;
    let expect = expect![[
        r#"OK ("\ndbus (send)\nbus=session,\n" "\ndbus (send)\n    bus=session,\n" 31 0)"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_indents_open_and_close_blocks_from_upstream_contract() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert "\n{\ndbus (send)\nbus=session,\n}")
         (indent-region (point-min) (point-max))
         (list (buffer-string)
               (mapcar
                (lambda (line)
                  (goto-char (point-min))
                  (forward-line line)
                  (current-indentation))
                '(0 1 2 3 4))))"##;
    let expect = expect![[r#"OK ("\n{\n  dbus (send)\n      bus=session,\n}" (0 0 2 6 0))"#]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_preserves_aare_alternation_indentation() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert "\n{\n  /dev/{,urandom,null} r,\n}")
         (goto-char (point-min))
         (forward-line 2)
         (let ((before-point (point)))
           (indent-region (point-min) (point-max))
           (list (buffer-string)
                 before-point (point)
                 (current-indentation))))"##;
    let expect = expect![[r#"OK ("\n{\n  /dev/{,urandom,null} r,\n}" 4 4 2)"#]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_indents_file_rules_and_profile_transitions_practically() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert
          "\n{\nfile Cx /usr/libexec/rygel/mx-extract -> mx-extract,\n"
          "file mrix /usr/lib/@{multiarch}/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner,\n"
          "file r /usr/share/gupnp-dlna-2.0/dlna-profiles/{,*},\n}")
         (indent-region (point-min) (point-max))
         (list (buffer-string)
               (apparmor-mode--block-depth)))"##;
    let expect = expect![[
        r#"OK ("\n{\n  file Cx /usr/libexec/rygel/mx-extract -> mx-extract,\n  file mrix /usr/lib/@{multiarch}/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner,\n  file r /usr/share/gupnp-dlna-2.0/dlna-profiles/{,*},\n}" 1)"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_indents_profile_with_flags_and_body_rules() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert
          "\nprofile busybox /usr/bin/busybox flags=(unconfined) {\n"
          "userns,\n/usr/bin/busybox mr,\n}")
         (indent-region (point-min) (point-max))
         (list (buffer-string)
               (mapcar
                (lambda (line)
                  (goto-char (point-min))
                  (forward-line line)
                  (current-indentation))
                '(1 2 3 4))))"##;
    let expect = expect![[
        r#"OK ("\nprofile busybox /usr/bin/busybox flags=(unconfined) {\n  userns,\n  /usr/bin/busybox mr,\n}" (0 2 2 0))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_indents_file_rules_with_variable_references() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert
          "\n{\n/usr/lib/@{multiarch}/gconv/*.so mr,\n"
          "@{etc_ro}/ld.so.cache mr,\n@{HOME}/.config r,\n}")
         (indent-region (point-min) (point-max))
         (list (buffer-string)
               (apparmor-mode--block-depth)))"##;
    let expect = expect![[
        r#"OK ("\n{\n  /usr/lib/@{multiarch}/gconv/*.so mr,\n  @{etc_ro}/ld.so.cache mr,\n  @{HOME}/.config r,\n}" 1)"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_indents_network_rule_series_inside_profile() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert
          "\n{\nnetwork inet dgram,\nnetwork inet6 dgram,\n"
          "network inet stream,\n}")
         (indent-region (point-min) (point-max))
         (list (buffer-string)
               (mapcar
                (lambda (line)
                  (goto-char (point-min))
                  (forward-line line)
                  (current-indentation))
                '(2 3 4))))"##;
    let expect = expect![[
        r#"OK ("\n{\n  network inet dgram,\n  network inet6 dgram,\n  network inet stream,\n}" (2 2 2))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_indents_capability_rule_series_inside_profile() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert
          "\n{\ncapability dac_override,\n"
          "capability dac_read_search,\n}")
         (indent-region (point-min) (point-max))
         (list (buffer-string)
               (mapcar
                (lambda (line)
                  (goto-char (point-min))
                  (forward-line line)
                  (current-indentation))
                '(2 3))))"##;
    let expect = expect![[
        r#"OK ("\n{\n  capability dac_override,\n  capability dac_read_search,\n}" (2 2))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_indents_real_multiline_dbus_continuations() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert
          "\n{\n  dbus send\n       bus=session\n"
          "       path=/org/freedesktop/DBus\n"
          "       interface=org.freedesktop.DBus\n"
          "       member=Hello\n"
          "       peer=(name=org.freedesktop.DBus),\n}")
         (indent-region (point-min) (point-max))
         (list (buffer-string)
               (mapcar
                (lambda (line)
                  (goto-char (point-min))
                  (forward-line line)
                  (current-indentation))
                '(2 3 4 5 6 7))))"##;
    let expect = expect![[
        r#"OK ("\n{\n  dbus send\n      bus=session\n      path=/org/freedesktop/DBus\n      interface=org.freedesktop.DBus\n      member=Hello\n      peer=(name=org.freedesktop.DBus),\n}" (2 6 6 6 6 6))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_indents_nested_profiles_and_balanced_closers() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert
          "\nprofile outer /usr/bin/outer {\n"
          "allow file rwlkm /{**,},\n"
          "profile inner /usr/bin/inner {\n"
          "allow network,\n}\n}")
         (indent-region (point-min) (point-max))
         (list
          (buffer-string)
          (mapcar
           (lambda (line)
             (goto-char (point-min))
             (forward-line line)
             (list (current-indentation)
                   (apparmor-mode--block-depth)))
           '(1 2 3 4 5 6))))"##;
    let expect = expect![[
        r#"OK ("\nprofile outer /usr/bin/outer {\n  allow file rwlkm /{**,},\n  profile inner /usr/bin/inner {\n    allow network,\n  }\n}" ((0 0) (2 1) (2 1) (4 2) (2 2) (0 1)))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

use expect_test::expect;

use super::assert_apparmor_mode_parity;

#[test]
fn apparmor_mode_completes_real_local_include_tree_deterministically() {
    let elisp_form = r##"(let* ((default-directory
                 (file-name-as-directory
                  (expand-file-name "apparmor-completion/" (getenv "HOME"))))
                (local-dir
                 (expand-file-name "fixtures/policy/local/" default-directory)))
         (make-directory local-dir t)
         (dolist (name '("service-base" "service-extra" "socket-rule"))
           (with-temp-file (expand-file-name name local-dir)
             (insert "# deterministic fixture\n")))
         (list
          (sort (apparmor-mode-complete-include
                 "fixtures/policy/local/service" t)
                #'string<)
          (apparmor-mode-complete-include
           "fixtures/policy/local/missing" t)))"##;
    let expect = expect![[
        r#"OK (("fixtures/policy/local/service-base" "fixtures/policy/local/service-extra") nil)"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_routes_system_include_completion_to_exact_directory() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'file-name-all-completions)
               (lambda (file directory)
                 (push (list file directory) calls)
                 '("base" "base-extra"))))
           (list
            (apparmor-mode-complete-include
             "abstractions/base")
            (apparmor-mode-complete-include
             "tunables/glob")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("abstractions/base" "abstractions/base-extra") ("tunables/base" "tunables/base-extra") (("base" "/etc/apparmor.d/abstractions/") ("glob" "/etc/apparmor.d/tunables/")))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_keyword_capf_reports_bounds_candidates_and_metadata() {
    let elisp_form = r##"(with-temp-buffer
         (apparmor-mode)
         (insert "capab")
         (goto-char (point-max))
         (let ((capf (apparmor-mode-completion-at-point)))
           (list
            (buffer-substring-no-properties
             (nth 0 capf) (nth 1 capf))
            (nth 0 capf)
            (nth 1 capf)
            (nth 2 capf)
            (plist-get (nthcdr 3 capf) :company-docsig)
            (funcall
             (plist-get (nthcdr 3 capf) :company-docsig)
             "capability"))))"##;
    let expect = expect![[
        r#"OK ("capab" 1 6 ("all" "audit" "capability" "chmod" "delegate" "dbus" "deny" "file" "flags" "io_uring" "include" "include if exists" "link" "mount" "mqueue" "network" "on" "owner" "pivot_root" "profile" "quiet" "remount" "rlimit" "safe" "subset" "to" "umount" "unsafe" "userns") identity "capability")"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

#[test]
fn apparmor_mode_include_capf_distinguishes_local_and_system_paths() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'apparmor-mode-complete-include)
               (lambda (prefix &optional local)
                 (push (list prefix local) calls)
                 (if local
                     '("local/service" "local/service-extra")
                   '("abstractions/base"
                     "abstractions/base-nameservice")))))
           (let ((results
                  (mapcar
                   (lambda (text)
                     (with-temp-buffer
                       (apparmor-mode)
                       (insert text)
                       (goto-char (point-max))
                       (let ((capf
                              (apparmor-mode-completion-at-point)))
                         (list
                          (buffer-substring-no-properties
                           (nth 0 capf) (nth 1 capf))
                          (nth 2 capf)))))
                   '("include <abstractions/base"
                     "include \"local/service"))))
             (list results (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ((("base" ("abstractions/base" "abstractions/base-nameservice")) ("service" ("local/service" "local/service-extra"))) (("base" nil) ("service" t)))"#
    ]];
    assert_apparmor_mode_parity(elisp_form, expect);
}

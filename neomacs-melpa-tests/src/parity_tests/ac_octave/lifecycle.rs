use expect_test::expect;

use super::assert_ac_octave_parity;

#[test]
fn ac_octave_init_starts_octave_then_synchronizes_a_readable_directory_exactly() {
    let elisp_form = r##"(let ((default-directory
                    "/workspace/project/")
                   calls)
               (cl-letf
                   (((symbol-function
                      'run-octave)
                     (lambda (&optional background)
                       (push
                        (list
                         'run
                         background)
                        calls)
                       'run-result))
                    ((symbol-function
                      'file-readable-p)
                     (lambda (path)
                       (push
                        (list
                         'readable
                         path)
                        calls)
                       t))
                    ((symbol-function
                      'inferior-octave-send-list-and-digest)
                     (lambda (commands)
                       (push
                        (list
                         'send
                         commands)
                        calls)
                       'send-result)))
                 (list
                  (ac-octave-init)
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (send-result ((run t) (readable "/workspace/project/") (send ("cd /workspace/project/;\n"))))"#
    ]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_init_still_starts_octave_but_skips_cd_for_an_unreadable_directory() {
    let elisp_form = r##"(let ((default-directory
                    "/missing/")
                   calls)
               (cl-letf
                   (((symbol-function
                      'run-octave)
                     (lambda (&optional background)
                       (push
                        (list
                         'run
                         background)
                        calls)))
                    ((symbol-function
                      'file-readable-p)
                     (lambda (path)
                       (push
                        (list
                         'readable
                         path)
                        calls)
                       nil))
                    ((symbol-function
                      'inferior-octave-send-list-and-digest)
                     (lambda (commands)
                       (push
                        (list
                         'unexpected-send
                         commands)
                        calls))))
                 (list
                  (ac-octave-init)
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil ((run t) (readable "/missing/")))"#]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_load_selects_octave_inf_only_for_emacs_24_3_and_older() {
    let elisp_form = r##"(mapcar
               (lambda (version)
                 (let ((emacs-major-version
                        (car version))
                       (emacs-minor-version
                        (cdr version))
                       calls)
                   (cl-letf
                       (((symbol-function
                          'require)
                         (lambda (feature
                                  &optional
                                  _filename
                                  _noerror)
                           (push feature calls)
                           feature)))
                     (load
                      (getenv
                       "NEOMACS_PACKAGE_SOURCE")
                      nil t t)
                     (list
                      version
                      (nreverse calls)))))
               '((24 . 3)
                 (24 . 4)
                 (25 . 0)))"##;
    let expect = expect![[
        r#"OK (((24 . 3) (cl auto-complete octave-inf)) ((24 . 4) (cl auto-complete octave)) ((25 . 0) (cl auto-complete octave)))"#
    ]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_setup_keeps_an_existing_source_position_and_returns_the_live_list() {
    let elisp_form = r##"(let ((ac-sources
                    '(first
                      ac-source-octave
                      last)))
               (list
                (ac-octave-setup)
                ac-sources
                (ac-octave-setup)
                ac-sources))"##;
    let expect = expect![[r#"OK (#1=(first ac-source-octave last) #1# #1# #1#)"#]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_setup_prepends_to_a_nonempty_source_list_and_preserves_existing_order() {
    let elisp_form = r##"(let ((ac-sources
                    '(first
                      last)))
               (let ((return
                      (ac-octave-setup)))
                 (list
                  return
                  ac-sources
                  (eq
                   return
                   ac-sources))))"##;
    let expect = expect![[r#"OK (#1=(ac-source-octave first last) #1# t)"#]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_setup_adds_source_to_an_empty_buffer_local_configuration() {
    let elisp_form = r##"(with-temp-buffer
               (setq-local
                ac-sources
                nil)
               (list
                (local-variable-p
                 'ac-sources)
                (ac-octave-setup)
                ac-sources
                (local-variable-p
                 'ac-sources)))"##;
    let expect = expect![[r#"OK (t #1=(ac-source-octave) #1# t)"#]];

    assert_ac_octave_parity(elisp_form, expect);
}

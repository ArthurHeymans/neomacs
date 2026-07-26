use expect_test::expect;

use super::assert_ac_racer_parity;

#[test]
fn ac_racer_callable_variable_group_and_auto_complete_source_metadata_match() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (function)
                  (list
                   function
                   (help-function-arglist
                    function t)
                   (interactive-form
                    function)
                   (documentation
                    function)
                   (file-name-nondirectory
                    (symbol-file
                     function
                     'defun))))
                '(ac-racer--collect-candidates
                  ac-racer--prefix
                  ac-racer--candidates
                  ac-racer-setup))
               (list
                (get
                 'ac-racer--tempfile
                 'standard-value)
                (get
                 'ac-racer--tempfile
                 'variable-documentation)
                (default-boundp
                 'ac-racer--tempfile))
               (list
                (get
                 'ac-racer
                 'group-documentation)
                (get
                 'ac-racer
                 'custom-prefix)
                (get
                 'ac-racer
                 'custom-links)
                (assq
                 'ac-racer
                 (get
                  'auto-complete
                  'custom-group)))
               (copy-tree
                ac-source-racer)
               (mapcar
                (lambda (key)
                  (cons
                   key
                   (cdr
                    (assq
                     key
                     ac-source-racer))))
                '(prefix
                  candidates
                  requires)))"##;
    let expect = expect![[
        r#"OK (((ac-racer--collect-candidates nil nil nil "ac-racer.el") (ac-racer--prefix nil nil nil "ac-racer.el") (ac-racer--candidates nil nil nil "ac-racer.el") (ac-racer-setup nil (interactive nil) nil "ac-racer.el")) (nil nil t) ("auto-complete source of racer" nil nil (ac-racer custom-group)) ((prefix . ac-racer--prefix) (candidates . ac-racer--candidates) (requires . -1)) ((prefix . ac-racer--prefix) (candidates . ac-racer--candidates) (requires . -1)))"#
    ]];

    assert_ac_racer_parity(elisp_form, expect);
}

#[test]
fn ac_racer_packaged_source_descriptor_readme_and_autoload_assets_have_exact_hashes() {
    let elisp_form = r##"(let ((root
                    (file-name-directory
                     (symbol-file
                      'ac-racer-setup
                      'defun))))
               (mapcar
                (lambda (file)
                  (let ((path
                         (expand-file-name
                          file
                          root)))
                    (with-temp-buffer
                      (insert-file-contents-literally
                       path)
                      (list
                       file
                       (buffer-size)
                       (secure-hash
                        'sha256
                        (current-buffer))))))
                '("ac-racer.el"
                  "ac-racer-pkg.el"
                  "README-elpa"
                  "ac-racer-autoloads.el")))"##;
    let expect = expect![[
        r#"OK (("ac-racer.el" 2963 "69da8567f2cad00c5e696b7178e04bb47790d9a2939ded7f83db47d835c9c455") ("ac-racer-pkg.el" 455 "b3e554487bb1efd18cc8b9911bd61878c61ac4aa827989d031da7812401def6f") ("README-elpa" 73 "186e2527d27dd774591673e833b4096980e246f2448321c36f49de0cf575b290") ("ac-racer-autoloads.el" 728 "659fb80b4e5ef4b071d5a0be7689b59ffdb444aec2a1891ddba289d504c3c40c"))"#
    ]];

    assert_ac_racer_parity(elisp_form, expect);
}

#[test]
fn ac_racer_source_definition_cells_remain_runtime_rebindable() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'ac-racer--prefix)
                     (lambda ()
                       (push
                        'prefix
                        calls)
                       17))
                    ((symbol-function
                      'ac-racer--candidates)
                     (lambda ()
                       (push
                        'candidates
                        calls)
                       '("one"
                         "two"))))
                 (list
                  (funcall
                   (cdr
                    (assq
                     'prefix
                     ac-source-racer)))
                  (funcall
                   (cdr
                    (assq
                     'candidates
                     ac-source-racer)))
                  (nreverse calls)
                  (cdr
                   (assq
                    'requires
                    ac-source-racer)))))"##;
    let expect = expect![[r#"OK (17 ("one" "two") (prefix candidates) -1)"#]];

    assert_ac_racer_parity(elisp_form, expect);
}

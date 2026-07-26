use expect_test::expect;

use super::assert_accent_parity;

#[test]
fn accent_packaged_source_descriptor_autoload_and_readme_assets_have_exact_hashes() {
    let elisp_form = r##"(let ((root
                    (file-name-directory
                     (symbol-file
                      'accent-lst
                      'defun))))
               (mapcar
                (lambda (file)
                  (with-temp-buffer
                    (insert-file-contents-literally
                     (expand-file-name
                      file
                      root))
                    (list
                     file
                     (buffer-size)
                     (secure-hash
                      'sha256
                      (current-buffer)))))
                '("accent.el"
                  "accent-pkg.el"
                  "accent-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("accent.el" 6265 "9e3b1d1d73b22539e42e4c369fcb1f703d7a676d93bb3c02e3baf68facc52a4f") ("accent-pkg.el" 443 "606707585d2f8352bfeb48a1c9401ee33dfd33ea76dcf740a689a5dc3d9bac47") ("accent-autoloads.el" 1102 "c416fa1364e2ffe01cab73f314178e6faafd2072a82917bc2542ad194c7ea4c6") ("README-elpa" 374 "9b08cfd4db9c9e3e44a61773dfb0910e4cd89a0970248a489f7d2def919aadf7"))"#
    ]];

    assert_accent_parity(elisp_form, expect);
}

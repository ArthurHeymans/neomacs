use expect_test::expect;

use super::assert_academic_phrases_parity;

#[test]
fn academic_phrases_packaged_source_descriptor_autoload_and_readme_assets_have_exact_hashes() {
    let elisp_form = r##"(let ((root
                    (file-name-directory
                     (symbol-file
                      'academic-phrases--insert
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
                '("academic-phrases.el"
                  "academic-phrases-pkg.el"
                  "academic-phrases-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("academic-phrases.el" 150714 "6adbf06c706edbbecc40a37277634f6597ae3fcc14f08f0c73862a85b2b8161f") ("academic-phrases-pkg.el" 568 "1fb5d11f726361b536c8e6a1531a3ab46fb15afc7fdf20fe64dcd5e34a367e56") ("academic-phrases-autoloads.el" 975 "16dd9936901ba4da9d87212f16ae166f10e10878eb0fcd032310ea9a67f70f27") ("README-elpa" 808 "59447e86eb0d2f6be20fb79a1c8ccd32d426d24fbfcdd5dedcb0456a872136d7"))"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

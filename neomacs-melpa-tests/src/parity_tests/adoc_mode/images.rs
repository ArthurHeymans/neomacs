use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_image_attribute_resolution_handles_defined_nested_unknown_and_plain_paths() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          ":imagesdir: assets\n"
          ":format: svg\n"
          ":compound.name: nested\n"
          ":empty: \n\n")
         (mapcar
          #'adoc--resolve-attribute-references
          '("{imagesdir}/logo.{format}"
            "{compound.name}/x.png"
            "{unknown}/x.png"
            "plain/path.png"
            "{imagesdir}/{unknown}/{format}")))"##;
    let expect = expect![[
        r#"OK ("assets/logo.svg" "nested/x.png" "{unknown}/x.png" "plain/path.png" "assets/{unknown}/svg")"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_image_link_parser_bounds_struct_and_point_fallback_match() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "before image:one.png[One] middle "
          "image::two.svg[Two,300,200] after")
         (adoc-mode)
         (mapcar
          (lambda (needle)
            (goto-char (point-min))
            (search-forward needle)
            (let* ((location (match-beginning 0))
                   (bounds (adoc-bounds-of-image-link-at location))
                   (link (adoc-image-link-at location)))
              (list
               bounds
               (and bounds
                    (buffer-substring-no-properties
                     (car bounds) (cdr bounds)))
               (and link
                    (list
                     (adoc-image-link-begin link)
                     (adoc-image-link-end link)
                     (adoc-image-link-uri link)
                     (adoc-image-link-begin-uri link)
                     (adoc-image-link-end-uri link)
                     (adoc-image-link-begin-attributes link)
                     (adoc-image-link-end-attributes link))))))
          '("one.png" "two.svg" "before")))"##;
    let expect = expect![[
        r#"OK (((8 . 26) "image:one.png[One]" (8 26 "one.png" 14 21 21 26)) ((34 . 61) "image::two.svg[Two,300,200]" (34 61 "two.svg" 41 48 48 61)) (nil nil nil))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_remote_image_cache_protocol_filter_and_overlay_lifecycle_match() {
    let elisp_form = r##"(let ((adoc--remote-image-cache
                (make-hash-table :test 'equal))
               (copy-count 0)
               (create-count 0))
         (cl-letf
             (((symbol-function 'url-copy-file)
               (lambda (_url path &optional _ok)
                 (setq copy-count (1+ copy-count))
                 (with-temp-file path (insert "image"))))
              ((symbol-function 'file-exists-p)
               (lambda (_file) t))
              ((symbol-function 'image-type-available-p)
               (lambda (_type) nil))
              ((symbol-function 'create-image)
               (lambda (file &rest _args)
                 (setq create-count (1+ create-count))
                 (list 'image :file file)))
              ((symbol-function 'display-images-p)
               (lambda () t)))
           (let ((first
                  (adoc--get-remote-image
                   "https://example.test/a.png"))
                 second)
             (setq second
                   (adoc--get-remote-image
                    "https://example.test/a.png"))
             (with-temp-buffer
               (insert
                "image:local.png[Local]\n"
                "image::https://example.test/remote.png[Remote]\n"
                "image::http://example.test/blocked.png[Blocked]\n")
               (let ((adoc-display-remote-images t)
                     (adoc-remote-image-protocols '("https")))
                 (adoc-display-images)
                 (let ((overlays
                        (mapcar
                         (lambda (overlay)
                           (list
                            (overlay-start overlay)
                            (overlay-end overlay)
                            (overlay-get overlay 'adoc-image)
                            (overlay-get overlay 'face)
                            (keymapp (overlay-get overlay 'keymap))))
                         (adoc-image-overlays))))
                   (adoc-toggle-images)
                   (let ((after-remove
                          (length (adoc-image-overlays))))
                     (adoc-toggle-images)
                     (list
                      (equal first second)
                      copy-count
                      create-count
                      overlays
                      after-remove
                      (length (adoc-image-overlays))))))))))"##;
    let expect =
        expect!["OK (t 1 6 ((71 118 t default t) (24 70 t default t) (1 23 t default t)) 0 3)"];
    assert_adoc_mode_parity(elisp_form, expect);
}

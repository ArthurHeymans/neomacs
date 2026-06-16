//! Complex combo batch 177 — `image-type-available-p` matrix expanded,
//! `image-size` for display variants, image-cache, image-transforms.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx177_image_type_available_p_full_matrix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (mapcar (lambda (t) (list t (image-type-available-p t)))
            '(png jpeg jpg gif tiff xpm xbm svg imagemagick webp pbm))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx177_image_create_with_image_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((img (create-image "test.xpm" 'xpm nil :ascent 'center)))
      (list (imagep img)
            (car img)
            (plist-get (cdr img) :type)
            (plist-get (cdr img) :file)
            (plist-get (cdr img) :ascent)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx177_image_cache_eviction_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (boundp 'image-cache-eviction-delay)
          (integerp image-cache-eviction-delay)
          (fboundp 'clear-image-cache))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx177_image_transforms_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (list (fboundp 'image-transforms-p)
          (when (fboundp 'image-transforms-p) (image-transforms-p)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx177_image_size_query_with_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((img (create-image "non-existent.png" 'png nil)))
      (list (imagep img)
            (condition-case err
                (image-size img)
              (error :err))
            (condition-case err
                (image-size img t)
              (error :err))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx177_image_mask_p_predicate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((img (create-image "non-existent.png" 'png nil :mask 'heuristic)))
      (list (imagep img)
            (plist-get (cdr img) :mask)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx177_image_animate_predicate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((img (create-image "non-existent.gif" 'gif nil)))
      (list (imagep img)
            (condition-case err (image-animated-p img) (error :err))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx177_image_flush_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((img (create-image "non-existent.png" 'png nil)))
      (list (fboundp 'image-flush)
            (imagep img)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx177_create_image_with_data_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((fake-data (unibyte-string #x89 #x50 #x4e #x47 #x0d #x0a #x1a #x0a)))
      (let ((img (create-image fake-data 'png t)))
        (list (imagep img)
              (plist-get (cdr img) :type))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx177_image_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((img (create-image "non-existent.png" 'png nil)))
      (with-temp-buffer
        (buffer-enable-undo)
        (insert "Image mega test buffer content")
        (put-text-property 1 6 'face 'bold)
        (put-text-property 7 12 'display img)
        (let ((m (set-marker (make-marker) 8))
              (ov (make-overlay 4 14)))
          (overlay-put ov 'face 'italic)
          (overlay-put ov 'evaporate t)
          (narrow-to-region 2 18)
          (let ((state (list (imagep img)
                             (imagep (get-text-property 7 'display))
                             (buffer-string)
                             (marker-position m)
                             (overlay-start ov) (overlay-end ov)
                             (text-properties-at 1))))
            (undo)
            (widen)
            (list state (buffer-string) (marker-position m)
                  (overlay-start ov) (overlay-end ov)
                  (text-properties-at 1))))))
  (error (list :errored (car e))))
"##,
    );
}

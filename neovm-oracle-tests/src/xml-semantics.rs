//! Oracle parity tests for GNU libxml primitive validation semantics.
//!
//! GNU implements `libxml-parse-xml-region` and `libxml-parse-html-region` in
//! `src/xml.c`.  Their shared `parse_region` defaults nil bounds, calls
//! `validate_region`, and only then validates non-nil BASE-URL as a string.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_libxml_parse_region_validates_region_before_base_url_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "<root/>")
  (list
   (condition-case err
       (libxml-parse-xml-region "x" "y" 42)
     (error (list (car err) (cdr err))))
   (condition-case err
       (libxml-parse-xml-region 99 100 42)
     (error (let ((data (cdr err)))
              (list (car err)
                    (length data)
                    (bufferp (car data))
                    (cadr data)
                    (caddr data)))))
   (condition-case err
       (libxml-parse-html-region "x" "y" 42)
     (error (list (car err) (cdr err))))
   (condition-case err
       (libxml-parse-html-region 1 8 42)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

//! coding-system-plist / coding-system-get / -category / -base parity for
//! utf-8 and its eol variants, latin-1, us-ascii; plus the no-conversion
//! plist :eol-type gap.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn cs_category_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (coding-system-category 'utf-8)
        (coding-system-category 'iso-8859-1)
        (coding-system-category 'raw-text))"##,
    );
}

#[test]
fn cs_get_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (coding-system-get 'utf-8 :mime-charset)
        (coding-system-get 'iso-8859-1 :mime-charset)
        (coding-system-get 'utf-16 :endian)
        (coding-system-doc-string 'utf-8))"##,
    );
}

#[test]
fn csplist_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((p (coding-system-plist 'utf-8)))
  (list (plist-get p :name) (plist-get p :mime-charset) (plist-get p :ascii-compatible-p)
        (plist-get p :category)))"##,
    );
}

#[test]
fn csplist_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (plist-get (coding-system-plist 'utf-8-unix) :eol-type)
        (plist-get (coding-system-plist 'utf-8-dos) :eol-type)
        (plist-get (coding-system-plist 'latin-1) :mime-charset)
        (plist-get (coding-system-plist 'us-ascii) :ascii-compatible-p))"##,
    );
}

#[test]
#[ignore = "DIVERGENCE: (coding-system-plist 'no-conversion) omits the :eol-type unix entry (and uses a truncated :docstring) that GNU includes."]
fn divergence_coding_plist_no_conversion_eol_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (plist-get (coding-system-plist 'no-conversion) :eol-type)
      (plist-member (coding-system-plist 'no-conversion) :eol-type)
      (plist-get (coding-system-plist 'raw-text) :eol-type))"##,
    );
}

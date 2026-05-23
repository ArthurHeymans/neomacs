//! Oracle parity tests for GNU `get-load-suffixes` semantics.
//!
//! GNU implements `get-load-suffixes` in `src/lread.c`.  With module support,
//! compressed representations listed in `jka-compr-load-suffixes` are skipped
//! for module suffixes, while still being tried for `.elc` and `.el`.

use super::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn oracle_get_load_suffixes_skips_compressed_module_representations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((load-suffixes (list module-file-suffix ".elc" ".el"))
      (load-file-rep-suffixes '("" ".gz" ".br"))
      (jka-compr-load-suffixes '(".gz")))
  (get-load-suffixes))
"#;

    assert_oracle_parity(form);
}

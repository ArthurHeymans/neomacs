//! Oracle parity tests for GNU `format-spec` semantics.
//!
//! GNU implements `format-spec` in `lisp/format-spec.el`.  The function is
//! used by packages such as Dired, Org, Tramp, ERC, and transient, so small
//! differences in missing-key, split, flag, or lazy substitution behavior are
//! observable in real configurations.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_format_spec_missing_and_quoted_percent_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'format-spec)
  (list
   (format-spec "known=%a pct=%%" '((?a . "A")))
   (condition-case err
       (format-spec "missing=%z" '((?a . "A")))
     (error (list (car err) (cadr err))))
   (format-spec "missing=%z pct=%%" '((?a . "A")) 'ignore)
   (format-spec "missing=%z pct=%%" '((?a . "A")) 'delete)
   (format-spec "missing=%z pct=%%" '((?a . "A")) 'keep-percent)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_format_spec_flags_width_precision_and_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'format-spec)
  (list
   (format-spec "[%8n]" '((?n . "abc")))
   (format-spec "[%-8n]" '((?n . "abc")))
   (format-spec "[%08n]" '((?n . "42")))
   (format-spec "[%^n]" '((?n . "AbC")))
   (format-spec "[%_n]" '((?n . "AbC")))
   (format-spec "[%.4n]" '((?n . "abcdef")))
   (format-spec "[%>4n]" '((?n . "abcdef")))
   (format-spec "[%<4n]" '((?n . "abcdef")))
   (format-spec "[%<06n]" '((?n . "abcdef")))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_format_spec_split_and_lazy_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'format-spec)
  (let ((calls 0))
    (list
     (format-spec "pre-%a-mid-%b-post"
                  `((?a . ,(lambda () (setq calls (1+ calls)) "A"))
                    (?b . ,(lambda () (setq calls (1+ calls)) "B"))))
     calls
     (format-spec "only-%a"
                  `((?a . ,(lambda () (setq calls (1+ calls)) "A"))
                    (?z . ,(lambda () (setq calls (1+ calls)) "Z"))))
     calls
     (format-spec "x%ay%b" '((?a . "A") (?b . "B")) nil t))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_format_spec_make_and_invalid_inputs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (require 'format-spec)
  (list
   (format-spec "%a:%b" (format-spec-make ?a "A" ?b "B"))
   (condition-case err
       (format-spec-make ?a "A" ?b)
     (error (list (car err) (cadr err))))
   (condition-case err
       (format-spec "%1" '((?a . "A")))
     (error (list (car err) (cadr err))))
   (condition-case err
       (format-spec "%@" '((?a . "A")))
     (error (list (car err) (cadr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

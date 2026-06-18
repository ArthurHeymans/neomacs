//! Complex combo batch 447 — 15 fun/utility probes: calc, info-lookup,
//! woman, ediff, smerge, vc, log-edit, copyright, time-stamp, hanoi,
//! life, rot13, studly, zone, doctor/eliza.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// calc basic operations.
#[test]
fn div_cx447_calc_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'calc)
  (list (condition-case e (calc-eval "2+3") (error (car e)))
        (fboundp 'calc-eval)))"##,
    );
}

/// info-lookup-symbol: looking up symbol documentation.
#[test]
fn div_cx447_info_lookup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'info-look)
  (list (fboundp 'info-lookup-symbol)
        (boundp 'info-lookup-mode)))"##,
    );
}

/// woman: browse manual pages without man.
#[test]
fn div_cx447_woman_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'woman)
  (list (fboundp 'woman) (fboundp 'woman-replace-last-win-file)))"##,
    );
}

/// ediff basics: file comparison.
#[test]
fn div_cx447_ediff_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'ediff)
  (list (boundp 'ediff-version) (fboundp 'ediff-files)))"##,
    );
}

/// diff: comparing buffer regions.
#[test]
fn div_cx447_diff_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'diff)
  (with-temp-buffer
    (insert "a\nb\nc\n")
    (diff-mode)
    (list major-mode (derived-mode-p 'diff-mode))))"##,
    );
}

/// vc: version control basics.
#[test]
fn div_cx447_vc_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'vc)
  (list (boundp 'vc-handled-backends) (fboundp 'vc-next-action)))"##,
    );
}

/// copyright: copyright update.
#[test]
fn div_cx447_copyright_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'copyright)
  (list (boundp 'copyright-limit) (fboundp 'copyright-update)))"##,
    );
}

/// time-stamp: time stamp formatting.
#[test]
fn div_cx447_time_stamp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'time-stamp)
  (stringp (time-stamp-string)))"##,
    );
}

/// hanoi: towers of Hanoi.
#[test]
fn div_cx447_hanoi_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'hanoi)
  (fboundp 'hanoi))"##,
    );
}

/// life: game of life.
#[test]
fn div_cx447_life_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'life)
  (fboundp 'life))"##,
    );
}

/// rot13: rotation cipher.
#[test]
fn div_cx447_rot13_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (rot13 "hello world")
      (rot13 "uryyb jbeyq"))"##,
    );
}

/// studly: studly caps transformation.
#[test]
fn div_cx447_studly_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'studly)
  (studlify-region-or-word))"##,
    );
}

/// zone: zone out effect.
#[test]
fn div_cx447_zone_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'zone)
  (fboundp 'zone))"##,
    );
}

/// doctor: emacs psychiatrist.
#[test]
fn div_cx447_doctor_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'doctor)
  (list (boundp 'doctor-doctors) (fboundp 'doctor)))"##,
    );
}

/// dunnet: emacs adventure game.
#[test]
fn div_cx447_dunnet_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'dunnet)
  (fboundp 'dunnet))"##,
    );
}

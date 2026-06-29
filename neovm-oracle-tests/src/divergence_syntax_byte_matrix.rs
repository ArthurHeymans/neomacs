//! Per-char *char-syntax* matrix (chars 0-255).
//!
//! One focused #[test] per char 0-255: query char-syntax.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_syntax_byte_0() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 0)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 1)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 2)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 3)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_4() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 4)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_5() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 5)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_6() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 6)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_7() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 7)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 8)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_9() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 9)",
        expect_test::expect![[r#""OK 32""#]],
    );
}

#[test]
fn div_syntax_byte_10() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 10)",
        expect_test::expect![[r#""OK 32""#]],
    );
}

#[test]
fn div_syntax_byte_11() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 11)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_12() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 12)",
        expect_test::expect![[r#""OK 32""#]],
    );
}

#[test]
fn div_syntax_byte_13() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 13)",
        expect_test::expect![[r#""OK 32""#]],
    );
}

#[test]
fn div_syntax_byte_14() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 14)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_15() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 15)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_16() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 16)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_17() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 17)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_18() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 18)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_19() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 19)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_20() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 20)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_21() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 21)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_22() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 22)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_23() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 23)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_24() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 24)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_25() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 25)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_26() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 26)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_27() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 27)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_28() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 28)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_29() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 29)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_30() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 30)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_31() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 31)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_32() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 32)",
        expect_test::expect![[r#""OK 32""#]],
    );
}

#[test]
fn div_syntax_byte_33() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 33)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_34() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 34)",
        expect_test::expect![[r#""OK 34""#]],
    );
}

#[test]
fn div_syntax_byte_35() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 35)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_36() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 36)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_37() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 37)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_38() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 38)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_39() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 39)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_40() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 40)",
        expect_test::expect![[r#""OK 40""#]],
    );
}

#[test]
fn div_syntax_byte_41() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 41)",
        expect_test::expect![[r#""OK 41""#]],
    );
}

#[test]
fn div_syntax_byte_42() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 42)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_43() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 43)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_44() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 44)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_45() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 45)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_46() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 46)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_47() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 47)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_48() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 48)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_49() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 49)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_50() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 50)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_51() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 51)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_52() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 52)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_53() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 53)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_54() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 54)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_55() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 55)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_56() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 56)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_57() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 57)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_58() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 58)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_59() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 59)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_60() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 60)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_61() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 61)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_62() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 62)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_63() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 63)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_64() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 64)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_65() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 65)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_66() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 66)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_67() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 67)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_68() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 68)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_69() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 69)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_70() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 70)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_71() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 71)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_72() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 72)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_73() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 73)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_74() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 74)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_75() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 75)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_76() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 76)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_77() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 77)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_78() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 78)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_79() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 79)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_80() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 80)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_81() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 81)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_82() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 82)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_83() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 83)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_84() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 84)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_85() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 85)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_86() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 86)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_87() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 87)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_88() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 88)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_89() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 89)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_90() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 90)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_91() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 91)",
        expect_test::expect![[r#""OK 40""#]],
    );
}

#[test]
fn div_syntax_byte_92() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 92)",
        expect_test::expect![[r#""OK 92""#]],
    );
}

#[test]
fn div_syntax_byte_93() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 93)",
        expect_test::expect![[r#""OK 41""#]],
    );
}

#[test]
fn div_syntax_byte_94() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 94)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_95() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 95)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_96() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 96)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_97() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 97)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_98() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 98)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_99() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 99)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_100() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 100)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_101() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 101)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_102() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 102)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_103() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 103)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_104() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 104)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_105() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 105)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_106() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 106)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_107() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 107)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_108() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 108)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_109() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 109)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_110() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 110)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_111() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 111)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_112() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 112)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_113() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 113)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_114() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 114)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_115() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 115)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_116() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 116)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_117() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 117)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_118() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 118)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_119() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 119)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_120() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 120)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_121() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 121)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_122() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 122)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_123() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 123)",
        expect_test::expect![[r#""OK 40""#]],
    );
}

#[test]
fn div_syntax_byte_124() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 124)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_125() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 125)",
        expect_test::expect![[r#""OK 41""#]],
    );
}

#[test]
fn div_syntax_byte_126() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 126)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_127() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 127)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_128() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 128)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_129() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 129)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_130() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 130)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_131() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 131)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_132() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 132)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_133() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 133)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_134() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 134)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_135() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 135)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_136() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 136)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_137() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 137)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_138() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 138)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_139() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 139)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_140() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 140)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_141() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 141)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_142() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 142)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_143() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 143)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_144() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 144)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_145() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 145)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_146() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 146)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_147() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 147)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_148() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 148)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_149() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 149)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_150() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 150)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_151() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 151)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_152() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 152)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_153() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 153)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_154() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 154)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_155() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 155)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_156() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 156)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_157() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 157)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_158() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 158)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_159() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 159)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_160() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 160)",
        expect_test::expect![[r#""OK 32""#]],
    );
}

#[test]
fn div_syntax_byte_161() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 161)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_162() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 162)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_163() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 163)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_164() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 164)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_165() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 165)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_166() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 166)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_167() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 167)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_168() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 168)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_169() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 169)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_170() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 170)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_171() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 171)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_172() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 172)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_173() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 173)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_174() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 174)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_175() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 175)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_176() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 176)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_177() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 177)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_178() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 178)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_179() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 179)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_180() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 180)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_181() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 181)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_182() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 182)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_183() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 183)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_184() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 184)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_185() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 185)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_186() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 186)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_187() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 187)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_188() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 188)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_189() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 189)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_190() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 190)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_191() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 191)",
        expect_test::expect![[r#""OK 46""#]],
    );
}

#[test]
fn div_syntax_byte_192() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 192)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_193() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 193)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_194() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 194)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_195() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 195)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_196() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 196)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_197() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 197)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_198() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 198)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_199() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 199)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_200() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 200)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_201() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 201)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_202() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 202)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_203() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 203)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_204() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 204)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_205() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 205)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_206() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 206)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_207() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 207)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_208() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 208)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_209() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 209)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_210() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 210)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_211() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 211)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_212() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 212)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_213() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 213)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_214() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 214)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_215() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 215)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_216() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 216)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_217() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 217)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_218() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 218)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_219() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 219)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_220() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 220)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_221() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 221)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_222() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 222)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_223() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 223)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_224() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 224)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_225() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 225)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_226() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 226)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_227() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 227)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_228() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 228)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_229() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 229)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_230() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 230)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_231() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 231)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_232() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 232)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_233() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 233)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_234() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 234)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_235() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 235)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_236() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 236)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_237() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 237)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_238() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 238)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_239() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 239)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_240() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 240)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_241() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 241)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_242() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 242)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_243() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 243)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_244() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 244)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_245() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 245)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_246() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 246)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_247() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 247)",
        expect_test::expect![[r#""OK 95""#]],
    );
}

#[test]
fn div_syntax_byte_248() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 248)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_249() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 249)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_250() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 250)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_251() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 251)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_252() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 252)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_253() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 253)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_254() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 254)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

#[test]
fn div_syntax_byte_255() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(char-syntax 255)",
        expect_test::expect![[r#""OK 119""#]],
    );
}

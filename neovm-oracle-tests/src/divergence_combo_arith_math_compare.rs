//! Divergence tests: arithmetic + number + math + comparison combos.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_bignum_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (let ((big (expt 2 64)))
    (list big
          (> big most-positive-fixnum)
          (= big (expt 2 64))
          (+ big 1)
          (= (+ big 1) (1+ (expt 2 64)))
          (* big 2)
          (= (* big 2) (expt 2 65))
          (/ big 2)
          (= (/ big 2) (expt 2 63))
          (% (1+ big) big)
          (= (% (1+ big) big) 1)))) "#,
    );
}

#[test]
fn divergence_float_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (list (= 0.0 -0.0)
        (eql 0.0 -0.0)
        (/= 1.0 1.0)
        (null (/= 1.0 1.0))
        (< 0.0 +0.0)
        (null (< 0.0 +0.0))
        (= +0.0 0.0)
        (> most-positive-fixnum 0)
        (< most-negative-fixnum 0)
        (floatp 1.5)
        (= (floor 3.7) 3)
        (= (ceiling 3.2) 4)
        (= (round 3.5) 4)
        (= (truncate 3.9) 3))) "#,
    );
}

#[test]
fn divergence_trig_log_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (let ((pi-val 3.141592653589793))
    (list (> (abs (- (sin 0.0) 0.0)) 1e-10)
          (null (> (abs (- (sin 0.0) 0.0)) 1e-10))
          (> (abs (- (cos 0.0) 1.0)) 1e-10)
          (null (> (abs (- (cos 0.0) 1.0)) 1e-10))
          (> (abs (- (exp 1.0) (exp 1.0))) 1e-10)
          (null (> (abs (- (exp 1.0) (exp 1.0))) 1e-10))
          (>= (log pi-val) 1.0)
          (>= (sqrt 4.0) 1.99)
          (= (sqrt 4.0) 2.0)
          (>= (abs pi-val) 3.14)))) "#,
    );
}

#[test]
fn divergence_bitwise_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (list (logand 15 6)
        (= (logand 15 6) 6)
        (logior 15 6)
        (= (logior 15 6) 15)
        (logxor 15 6)
        (= (logxor 15 6) 9)
        (lognot 0)
        (= (lognot 0) -1)
        (ash 1 4)
        (= (ash 1 4) 16)
        (ash 16 -2)
        (= (ash 16 -2) 4))) "#,
    );
}

#[test]
fn divergence_number_predicate_combos() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (list (numberp 42)
        (numberp 3.14)
        (numberp "hello")
        (null (numberp "hello"))
        (integerp 42)
        (null (integerp 3.14))
        (floatp 3.14)
        (null (floatp 42))
        (natnump 5)
        (natnump 0)
        (null (natnump -1))
        (zerop 0)
        (null (zerop 1))
        (plusp 5)
        (null (plusp -1))
        (minusp -3)
        (null (minusp 3)))) "#,
    );
}

#[test]
fn divergence_max_min_clamp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (list (max 1 5 3 9 2)
        (= (max 1 5 3 9 2) 9)
        (min 1 5 3 9 2)
        (= (min 1 5 3 9 2) 1)
        (max -1 -5 -3)
        (= (max -1 -5 -3) -1)
        (min -1 -5 -3)
        (= (min -1 -5 -3) -5)
        (cl-loop for x in '(1 5 3 9 2) maximize x into m finally return m)
        (= (cl-loop for x in '(1 5 3 9 2) maximize x into m finally return m) 9))) "#,
    );
}

#[test]
fn division_modulo_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (list (/ 10 3)
        (= (/ 10 3) 3)
        (% 10 3)
        (= (% 10 3) 1)
        (mod 10 3)
        (= (mod 10 3) 1)
        (mod -10 3)
        (= (mod -10 3) 2)
        (% -10 3)
        (= (% -10 3) -1)
        (/ -10 3)
        (= (/ -10 3) -3))) "#,
    );
}

#[test]
fn divergence_random_abs_comparison() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (list (abs -5)
        (= (abs -5) 5)
        (abs 3.14)
        (= (abs 3.14) 3.14)
        (abs -3.14)
        (= (abs -3.14) 3.14)
        (= (max (abs -10) (abs 5)) 10)
        (= (min (abs -10) (abs 5)) 5)
        (<= (random 100) 99)
        (>= (random 100) 0)
        (= (expt 2 10) 1024)
        (= (expt 10 3) 1000))) "#,
    );
}

#[test]
fn divergence_type_conversion_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (list (string-to-number "42")
        (= (string-to-number "42") 42)
        (string-to-number "3.14")
        (= (string-to-number "3.14") 3.14)
        (string-to-number "ff" 16)
        (= (string-to-number "ff" 16) 255)
        (string-to-number "101" 2)
        (= (string-to-number "101" 2) 5)
        (number-to-string 42)
        (string= (number-to-string 42) "42")
        (number-to-string 3.14)
        (string= (number-to-string 3.14) "3.14"))) "#,
    );
}

#[test]
fn divergence_cl_loop_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (list (cl-loop for i from 1 to 10 sum i)
        (= (cl-loop for i from 1 to 10 sum i) 55)
        (cl-loop for i from 1 to 10 count (cl-oddp i))
        (= (cl-loop for i from 1 to 10 count (cl-oddp i) 5)
        (cl-loop for i from 1 to 5 collect (* i i))
        (equal (cl-loop for i from 1 to 5 collect (* i i))
               '(1 4 9 16 25))
        (cl-loop for x in '(1 2 3 4 5) when (cl-evenp x) sum x)
        (= (cl-loop for x in '(1 2 3 4 5) when (cl-evenp x) sum x) 6))) "#,
    );
}

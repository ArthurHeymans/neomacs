//! Divergence tests: stress tests with large data, deep recursion, many objects.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_large_buffer_many_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz ERR (wrong-type-argument overlayp nil)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        "(progn
  (dotimes (_ 100) (insert \"abcdefghijklmnopqrstuvwxyz \"))
  (let ((count 0))
    (dotimes (i 50)
      (let ((ov (make-overlay (+ 1 (* i 27)) (+ 10 (* i 27)))))
        (overlay-put ov 'priority i)
        (overlay-put ov 'face (if (cl-evenp i) 'bold 'italic))
        (cl-incf count)))
    (list count
          (length (overlays-in 1 100))
          (overlay-get (car (overlays-at 50)) 'priority)
          (>= (length (overlays-in 1 (point-max))) 10)))) ",
        expect,
    );
}

#[test]
fn divergence_deep_recursive_accumulator() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (5050 125250 t)""#]];
    crate::common::assert_oracle_parity_expect(
        "(progn
  (defun test-deep-sum-xxx (n acc)
    (if (<= n 0) acc
      (test-deep-sum-xxx (1- n) (+ acc n))))
  (list (test-deep-sum-xxx 100 0)
        (test-deep-sum-xxx 500 0)
        (= (test-deep-sum-xxx 100 0) 5050))) ",
        expect,
    );
}

#[test]
fn divergence_many_interleaved_textprops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxOK (50 50 t t t t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        "(progn
  (insert (make-string 200 ?x))
  (dotimes (i 100)
    (put-text-property (1+ (* i 2)) (+ 2 (* i 2))
                       'idx i)
    (put-text-property (1+ (* i 2)) (+ 2 (* i 2))
                       'parity (if (cl-evenp i) 'even 'odd)))
  (let ((even-count 0) (odd-count 0))
    (dotimes (i 100)
      (if (eq (get-text-property (1+ (* i 2)) 'parity) 'even)
          (cl-incf even-count)
        (cl-incf odd-count)))
    (list even-count odd-count
          (= even-count 50)
          (= odd-count 50)
          (= (get-text-property 1 'idx) 0)
          (= (get-text-property 199 'idx) 99)))) ",
        expect,
    );
}

#[test]
fn divergence_large_list_map_filter_reduce() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (1000 500 t t t)""#]];
    crate::common::assert_oracle_parity_expect(
        "(let* ((nums (number-sequence 1 1000))
        (squares (mapcar (lambda (x) (* x x)) nums))
        (evens (seq-filter #'cl-evenp nums))
        (total (seq-reduce #'+ evens 0))
        (sum-sq (seq-reduce #'+ (seq-filter #'cl-evenp squares) 0)))
  (list (length nums)
        (length evens)
        (= total 250500)
        (= (nth 999 squares) 1000000)
        (> sum-sq 0))) ",
        expect,
    );
}

#[test]
fn divergence_many_nested_let_bindings() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (1 2 3 4 5 6 7 8 9 10 t t)""#]];
    crate::common::assert_oracle_parity_expect(
        "(let ((a 1))
  (let ((b (+ a 1)))
    (let ((c (+ b 1)))
      (let ((d (+ c 1)))
        (let ((e (+ d 1)))
          (let ((f (+ e 1)))
            (let ((g (+ f 1)))
              (let ((h (+ g 1)))
                (let ((i (+ h 1)))
                  (let ((j (+ i 1)))
                    (list a b c d e f g h i j
                          (= j 10)
                          (= (+ a b c d e f g h i j) 55)))))))))))) ",
        expect,
    );
}

#[test]
fn divergence_many_hash_table_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (200 1764 t missing t nil 0)""#]];
    crate::common::assert_oracle_parity_expect(
        "(let ((ht (make-hash-table :test 'equal :size 500)))
  (dotimes (i 200)
    (puthash (format \"key-%04d\" i) (* i i) ht))
  (list (hash-table-count ht)
        (gethash \"key-0042\" ht)
        (= (gethash \"key-0042\" ht) 1764)
        (gethash \"key-9999\" ht 'missing)
        (eq (gethash \"key-9999\" ht 'missing) 'missing)
        (dotimes (i 200) (remhash (format \"key-%04d\" i) ht))
        (hash-table-count ht))) ",
        expect,
    );
}

#[test]
fn divergence_large_string_search_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. The REDACTED REDACTED REDACTED jumps over the REDACTED dog. OK (200 t t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        "(progn
  (dotimes (_ 50)
    (insert \"The quick brown fox jumps over the lazy dog. \"))
  (goto-char 1)
  (let ((count 0))
    (while (re-search-forward \"\\\\(quick\\\\|lazy\\\\|brown\\\\|fox\\\\)\" nil t)
      (cl-incf count)
      (replace-match \"REDACTED\" t))
    (list count
          (>= count 100)
          (= (count-matches \"REDACTED\" 1 (point-max)) count)))) ",
        expect,
    );
}

#[test]
fn divergence_deep_catch_throw_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (5 7)""#]];
    crate::common::assert_oracle_parity_expect(
        "(catch 'done
  (dotimes (i 10)
    (catch (intern (format \"level-%d\" i))
      (dotimes (j 10)
        (when (and (= i 5) (= j 7))
          (throw 'done (list i j))))))) ",
        expect,
    );
}

#[test]
fn divergence_many_buffer_ops_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""START\nLine 000: \nLine 001: x\nLine 002: xx\nLine 003: xxx\nLine 004: xxxx\nLine 005: xxxxx\nLine 006: xxxxxx\nLine 007: xxxxxxx\nLine 008: xxxxxxxx\nLine 009: xxxxxxxxx\nLine 010: xxxxxxxxxx\nLine 011: xxxxxxxxxxx\nLine 012: xxxxxxxxxxxx\nLine 013: xxxxxxxxxxxxx\nLine 014: xxxxxxxxxxxxxx\nLine 015: xxxxxxxxxxxxxxx\nLine 016: xxxxxxxxxxxxxxxx\nLine 017: xxxxxxxxxxxxxxxxx\nLine 018: xxxxxxxxxxxxxxxxxx\nLine 019: xxxxxxxxxxxxxxxxxxx\nLine 020: \nLine 021: x\nLine 022: xx\nLine 023: xxx\nLine 024: xxxx\nLine 025: xxxxx\nLine 026: xxxxxx\nLine 027: xxxxxxx\nLine 028: xxxxxxxx\nLine 029: xxxxxxxxx\nLine 030: xxxxxxxxxx\nLine 031: xxxxxxxxxxx\nLine 032: xxxxxxxxxxxx\nLine 033: xxxxxxxxxxxxx\nLine 034: xxxxxxxxxxxxxx\nLine 035: xxxxxxxxxxxxxxx\nLine 036: xxxxxxxxxxxxxxxx\nLine 037: xxxxxxxxxxxxxxxxx\nLine 038: xxxxxxxxxxxxxxxxxx\nLine 039: xxxxxxxxxxxxxxxxxxx\nLine 040: \nLine 041: x\nLine 042: xx\nLine 043: xxx\nLine 044: xxxx\nLine 045: xxxxx\nLine 046: xxxxxx\nLine 047: xxxxxxx\nLine 048: xxxxxxxx\nLine 049: xxxxxxxxx\nLine 050: xxxxxxxxxx\nLine 051: xxxxxxxxxxx\nLine 052: xxxxxxxxxxxx\nLine 053: xxxxxxxxxxxxx\nLine 054: xxxxxxxxxxxxxx\nLine 055: xxxxxxxxxxxxxxx\nLine 056: xxxxxxxxxxxxxxxx\nLine 057: xxxxxxxxxxxxxxxxx\nLine 058: xxxxxxxxxxxxxxxxxx\nLine 059: xxxxxxxxxxxxxxxxxxx\nLine 060: \nLine 061: x\nLine 062: xx\nLine 063: xxx\nLine 064: xxxx\nLine 065: xxxxx\nLine 066: xxxxxx\nLine 067: xxxxxxx\nLine 068: xxxxxxxx\nLine 069: xxxxxxxxx\nLine 070: xxxxxxxxxx\nLine 071: xxxxxxxxxxx\nLine 072: xxxxxxxxxxxx\nLine 073: xxxxxxxxxxxxx\nLine 074: xxxxxxxxxxxxxx\nLine 075: xxxxxxxxxxxxxxx\nLine 076: xxxxxxxxxxxxxxxx\nLine 077: xxxxxxxxxxxxxxxxx\nLine 078: xxxxxxxxxxxxxxxxxx\nLine 079: xxxxxxxxxxxxxxxxxxx\nLine 080: \nLine 081: x\nLine 082: xx\nLine 083: xxx\nLine 084: xxxx\nLine 085: xxxxx\nLine 086: xxxxxx\nLine 087: xxxxxxx\nLine 088: xxxxxxxx\nLine 089: xxxxxxxxx\nLine 090: xxxxxxxxxx\nLine 091: xxxxxxxxxxx\nLine 092: xxxxxxxxxxxx\nLine 093: xxxxxxxxxxxxx\nLine 094: xxxxxxxxxxxxxx\nLine 095: xxxxxxxxxxxxxxx\nLine 096: xxxxxxxxxxxxxxxx\nLine 097: xxxxxxxxxxxxxxxxx\nLine 098: xxxxxxxxxxxxxxxxxx\nLine 099: xxxxxxxxxxxxxxxxxxxOK (101 t t 2055)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        "(progn
  (insert \"START\")
  (dotimes (i 100)
    (goto-char (point-max))
    (insert (format \"\\nLine %03d: %s\" i (make-string (mod i 20) ?x))))
  (goto-char 1)
  (let ((line-count 0))
    (while (not (eobp))
      (cl-incf line-count)
      (forward-line 1))
    (list line-count
          (>= line-count 100)
          (= (line-number-at-pos (point-max)) line-count)
          (buffer-size)))) ",
        expect,
    );
}

#[test]
fn divergence_many_undo_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect =
        expect_test::expect![[r#""BASE-0-1-2-3-4-5-6-7-8-9ERR (wrong-type-argument listp t)""#]];
    crate::common::assert_oracle_parity_expect(
        "(progn
  (insert \"BASE\")
  (dotimes (i 10)
    (undo-boundary)
    (goto-char (point-max))
    (insert (format \"-%d\" i)))
  (let ((s1 (buffer-string)))
    (dotimes (_ 5)
      (primitive-undo 1 buffer-undo-list))
    (let ((s2 (buffer-string)))
      (dotimes (_ 5)
        (primitive-undo 1 buffer-undo-list))
      (list s1 s2 (buffer-string))))) ",
        expect,
    );
}

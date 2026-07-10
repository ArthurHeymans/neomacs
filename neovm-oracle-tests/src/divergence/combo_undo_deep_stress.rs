//! Divergence tests: deep undo+replace+narrow+overlay+marker+textprop combos.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_undo_replace_narrow_overlay_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""QQQQRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRXXXERR (wrong-type-argument listp t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert (make-string 200 ?X))
  (dotimes (i 5)
    (let ((start (+ 1 (* i 40)))
          (end (+ 1 (* (+ i 1) 40))))
      (put-text-property start end 'zone (+ i 1))
      (let ((ov (make-overlay start end)))
        (overlay-put ov 'zone (+ i 1)))))
  (let ((m1 (copy-marker 1 t)) (m2 (copy-marker 41 t))
        (m3 (copy-marker 81 t)) (m4 (copy-marker 121 t))
        (m5 (copy-marker 161 t)))
    (undo-boundary)
    (narrow-to-region 41 120)
    (goto-char (point-min))
    (insert "QQQQ")
    (undo-boundary)
    (goto-char 1)
    (while (re-search-forward "XXXX" nil t)
      (replace-match "RRRR"))
    (let ((narrow-str (buffer-string)))
      (primitive-undo 2 buffer-undo-list)
      (widen)
      (list narrow-str
            (buffer-string)
            (= (buffer-size) 200)
            (= (marker-position m1) 1)
            (= (marker-position m2) 41)
            (= (marker-position m3) 81)
            (= (marker-position m4) 121)
            (= (marker-position m5) 161)
            (get-text-property 1 'zone) (= (get-text-property 1 'zone) 1)
            (get-text-property 41 'zone) (= (get-text-property 41 'zone) 2)
            (get-text-property 81 'zone) (= (get-text-property 81 'zone) 3)
            (get-text-property 121 'zone) (= (get-text-property 121 'zone) 4)
            (get-text-property 161 'zone) (= (get-text-property 161 'zone) 5))))) "#,
        expect,
    );
}

#[test]
fn divergence_multi_kill_ring_yank_undo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""ALPHA-BETA-GAMMA-DELTA-EPSILON-ZETA-ETA-THETAERR (args-out-of-range 43 47)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "ALPHA-BETA-GAMMA-DELTA-EPSILON-ZETA-ETA-THETA")
  (put-text-property 1 5 'greek 'a)
  (put-text-property 7 10 'greek 'b)
  (put-text-property 13 17 'greek 'g)
  (put-text-property 19 23 'greek 'd)
  (put-text-property 26 32 'greek 'e)
  (put-text-property 34 37 'greek 'z)
  (put-text-property 39 41 'greek 'et)
  (put-text-property 43 47 'greek 'th)
  (let ((ov (make-overlay 1 47))
        (m (copy-marker 7 t)))
    (overlay-put ov 'scope 'all)
    (undo-boundary)
    (kill-region 7 10)
    (undo-boundary)
    (kill-region 13 17)
    (undo-boundary)
    (kill-region 19 23)
    (undo-boundary)
    (goto-char 1)
    (yank)
    (undo-boundary)
    (goto-char (point-max))
    (yank)
    (let ((s (buffer-string)))
      (primitive-undo 5 buffer-undo-list)
      (list s
            (buffer-string)
            (string= (buffer-string)
                     "ALPHA-BETA-GAMMA-DELTA-EPSILON-ZETA-ETA-THETA")
            (marker-position m) (= (marker-position m) 7)
            (get-text-property 1 'greek) (eq (get-text-property 1 'greek) 'a)
            (get-text-property 7 'greek) (eq (get-text-property 7 'greek) 'b)
            (get-text-property 13 'greek) (eq (get-text-property 13 'greek) 'g)
            (get-text-property 19 'greek) (eq (get-text-property 19 'greek) 'd)
            (get-text-property 26 'greek) (eq (get-text-property 26 'greek) 'e)
            (overlay-get ov 'scope) (eq (overlay-get ov 'scope) 'all))))) "#,
        expect,
    );
}

#[test]
fn divergence_overlay_before_string_undo_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""AAAA\nXXXX\nCCCC\nYYYY\nEEEE\nZZZZ\nGGGG\nHHHHERR (wrong-type-argument listp t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "AAAA\nBBBB\nCCCC\nDDDD\nEEEE\nFFFF\nGGGG\nHHHH")
  (dotimes (i 8)
    (let ((start (+ 1 (* i 5)))
          (end (+ 4 (* i 5))))
      (put-text-property start (+ end 1) 'line (+ i 1))
      (let ((ov (make-overlay start (+ end 1))))
        (overlay-put ov 'before-string (format "[%d] " (+ i 1))))))
  (let ((m1 (copy-marker 1 t)) (m5 (copy-marker 21 t)))
    (undo-boundary)
    (goto-char 1)
    (re-search-forward "BBBB" nil t)
    (replace-match "XXXX")
    (undo-boundary)
    (goto-char 1)
    (re-search-forward "DDDD" nil t)
    (replace-match "YYYY")
    (undo-boundary)
    (goto-char 1)
    (re-search-forward "FFFF" nil t)
    (replace-match "ZZZZ")
    (let ((s (buffer-string)))
      (primitive-undo 3 buffer-undo-list)
      (list s
            (buffer-string)
            (string= (buffer-string)
                     "AAAA\nBBBB\nCCCC\nDDDD\nEEEE\nFFFF\nGGGG\nHHHH")
            (= (marker-position m1) 1)
            (= (marker-position m5) 21)
            (get-text-property 1 'line) (= (get-text-property 1 'line) 1)
            (get-text-property 6 'line) (= (get-text-property 6 'line) 2)
            (get-text-property 11 'line) (= (get-text-property 11 'line) 3)
            (get-text-property 16 'line) (= (get-text-property 16 'line) 4)
            (get-text-property 21 'line) (= (get-text-property 21 'line) 5)
            (get-text-property 26 'line) (= (get-text-property 26 'line) 6)
            (get-text-property 31 'line) (= (get-text-property 31 'line) 7)
            (get-text-property 36 'line) (= (get-text-property 36 'line) 8))))) "#,
        expect,
    );
}

#[test]
fn divergence_undo_transpose_regions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""BBA-AAB-DDC-CCD-FFE-EEF-GGG-HHH-III-JJJERR (wrong-type-argument listp t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "AAA-BBB-CCC-DDD-EEE-FFF-GGG-HHH-III-JJJ")
  (put-text-property 1 3 'idx 1)
  (put-text-property 5 7 'idx 2)
  (put-text-property 9 11 'idx 3)
  (put-text-property 13 15 'idx 4)
  (put-text-property 17 19 'idx 5)
  (put-text-property 21 23 'idx 6)
  (put-text-property 25 27 'idx 7)
  (put-text-property 29 31 'idx 8)
  (put-text-property 33 35 'idx 9)
  (put-text-property 37 39 'idx 10)
  (let ((ov1 (make-overlay 1 3)) (ov2 (make-overlay 5 7))
        (ov3 (make-overlay 9 11)) (ov4 (make-overlay 13 15))
        (ov5 (make-overlay 17 19)))
    (overlay-put ov1 'grp 'a) (overlay-put ov2 'grp 'b)
    (overlay-put ov3 'grp 'c) (overlay-put ov4 'grp 'd)
    (overlay-put ov5 'grp 'e)
    (let ((m (copy-marker 5 t)))
      (undo-boundary)
      (transpose-regions 1 3 5 7)
      (undo-boundary)
      (transpose-regions 9 11 13 15)
      (undo-boundary)
      (transpose-regions 17 19 21 23)
      (let ((s (buffer-string)))
        (primitive-undo 3 buffer-undo-list)
        (list s
              (buffer-string)
              (string= (buffer-string)
                       "AAA-BBB-CCC-DDD-EEE-FFF-GGG-HHH-III-JJJ")
              (= (marker-position m) 5)
              (get-text-property 1 'idx) (= (get-text-property 1 'idx) 1)
              (get-text-property 5 'idx) (= (get-text-property 5 'idx) 2)
              (get-text-property 9 'idx) (= (get-text-property 9 'idx) 3)
              (get-text-property 13 'idx) (= (get-text-property 13 'idx) 4)
              (get-text-property 17 'idx) (= (get-text-property 17 'idx) 5)
              (overlay-get ov1 'grp) (eq (overlay-get ov1 'grp) 'a)
              (overlay-get ov2 'grp) (eq (overlay-get ov2 'grp) 'b)))))) "#,
        expect,
    );
}

#[test]
fn divergence_undo_after_multiple_narrow_widen() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect =
        expect_test::expect![[r#""YYAAAXZZRRRXCCCXDDDXEEEERR (wrong-type-argument listp t)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "AAAXBBBXCCCXDDDXEEE")
  (put-text-property 1 3 'sec 'a)
  (put-text-property 5 7 'sec 'b)
  (put-text-property 9 11 'sec 'c)
  (put-text-property 13 15 'sec 'd)
  (put-text-property 17 19 'sec 'e)
  (let ((ov (make-overlay 1 19))
        (m (copy-marker 5 t)))
    (overlay-put ov 'wrap 'all)
    (undo-boundary)
    (narrow-to-region 5 15)
    (goto-char 5)
    (insert "ZZ")
    (undo-boundary)
    (widen)
    (narrow-to-region 1 7)
    (goto-char 1)
    (insert "YY")
    (undo-boundary)
    (widen)
    (goto-char 1)
    (re-search-forward "BBB" nil t)
    (replace-match "RRR")
    (let ((s (buffer-string)))
      (primitive-undo 3 buffer-undo-list)
      (list s
            (buffer-string)
            (string= (buffer-string) "AAAXBBBXCCCXDDDXEEE")
            (= (marker-position m) 5)
            (get-text-property 1 'sec) (eq (get-text-property 1 'sec) 'a)
            (get-text-property 5 'sec) (eq (get-text-property 5 'sec) 'b)
            (get-text-property 9 'sec) (eq (get-text-property 9 'sec) 'c)
            (get-text-property 13 'sec) (eq (get-text-property 13 'sec) 'd)
            (get-text-property 17 'sec) (eq (get-text-property 17 'sec) 'e)
            (overlay-get ov 'wrap) (eq (overlay-get ov 'wrap) 'all))))) "#,
        expect,
    );
}

#[test]
fn divergence_50_overlay_undo_stress() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPERR (wrong-type-argument listp t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert (make-string 250 ?M))
  (let ((ovs nil) (ms nil))
    (dotimes (i 50)
      (let* ((start (+ 1 (* i 5)))
             (end (min (+ start 4) 251))
             (ov (make-overlay start end)))
        (overlay-put ov 'idx (+ i 1))
        (push ov ovs)
        (put-text-property start end 'idx (+ i 1))
        (push (copy-marker start t) ms)))
    (setq ovs (nreverse ovs))
    (setq ms (nreverse ms))
    (undo-boundary)
    (goto-char 1)
    (while (re-search-forward "MMMMM" nil t)
      (replace-match "NNNNN"))
    (undo-boundary)
    (goto-char 1)
    (while (re-search-forward "NNNNN" nil t)
      (replace-match "PPPPP"))
    (let ((s (buffer-string)))
      (primitive-undo 2 buffer-undo-list)
      (let ((all-ok t))
        (dotimes (i 50)
          (when (or (not (= (overlay-get (nth i ovs) 'idx) (+ i 1)))
                    (not (= (get-text-property (+ 1 (* i 5)) 'idx) (+ i 1))))
            (setq all-ok nil)))
        (list s
              (buffer-string)
              (string= (buffer-string) (make-string 250 ?M))
              all-ok
              (= (buffer-size) 250)))))) "#,
        expect,
    );
}

#[test]
fn divergence_undo_delete_insert_interleaved() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""ONE-TWOPOINTFIFOURPOINTFIVE-THREE-FOUR-FIVE-SIX-SEVENERR (wrong-type-argument listp t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "ONE-TWO-THREE-FOUR-FIVE-SIX-SEVEN")
  (put-text-property 1 3 'num 1)
  (put-text-property 5 7 'num 2)
  (put-text-property 9 13 'num 3)
  (put-text-property 15 18 'num 4)
  (put-text-property 20 23 'num 5)
  (put-text-property 25 27 'num 6)
  (put-text-property 29 33 'num 7)
  (let ((ov (make-overlay 1 33))
        (m1 (copy-marker 5 t)) (m2 (copy-marker 15 t)))
    (overlay-put ov 'chain 'all)
    (undo-boundary)
    (delete-region 5 7)
    (undo-boundary)
    (goto-char 5)
    (insert "TWOPOINTFIVE")
    (undo-boundary)
    (delete-region 15 18)
    (undo-boundary)
    (goto-char 15)
    (insert "FOURPOINTFIVE")
    (let ((s (buffer-string)))
      (primitive-undo 4 buffer-undo-list)
      (list s
            (buffer-string)
            (string= (buffer-string) "ONE-TWO-THREE-FOUR-FIVE-SIX-SEVEN")
            (= (marker-position m1) 5)
            (= (marker-position m2) 15)
            (get-text-property 1 'num) (= (get-text-property 1 'num) 1)
            (get-text-property 5 'num) (= (get-text-property 5 'num) 2)
            (get-text-property 9 'num) (= (get-text-property 9 'num) 3)
            (get-text-property 15 'num) (= (get-text-property 15 'num) 4)
            (get-text-property 20 'num) (= (get-text-property 20 'num) 5)
            (get-text-property 25 'num) (= (get-text-property 25 'num) 6)
            (get-text-property 29 'num) (= (get-text-property 29 'num) 7)
            (overlay-get ov 'chain) (eq (overlay-get ov 'chain) 'all))))) "#,
        expect,
    );
}

#[test]
fn divergence_undo_with_buffer_substring() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""HEADER-ALPHA-BETA-GAMMA-DELTA-FOOTERERR (wrong-type-argument listp t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "HEADER-ALPHA-BETA-GAMMA-DELTA-FOOTER")
  (put-text-property 1 6 'role 'header)
  (put-text-property 8 12 'role 'body)
  (put-text-property 14 17 'role 'body)
  (put-text-property 19 23 'role 'body)
  (put-text-property 25 29 'role 'body)
  (put-text-property 31 36 'role 'footer)
  (let ((ov-h (make-overlay 1 6)) (ov-f (make-overlay 31 36))
        (m (copy-marker 8 t)))
    (overlay-put ov-h 'kind 'header)
    (overlay-put ov-f 'kind 'footer)
    (undo-boundary)
    (let ((sub (buffer-substring 8 29)))
      (delete-region 8 29)
      (undo-boundary)
      (goto-char 8)
      (insert (upcase sub))
      (let ((s (buffer-string)))
        (primitive-undo 2 buffer-undo-list)
        (list sub
              s
              (buffer-string)
              (string= (buffer-string)
                       "HEADER-ALPHA-BETA-GAMMA-DELTA-FOOTER")
              (= (marker-position m) 8)
              (get-text-property 1 'role) (eq (get-text-property 1 'role) 'header)
              (get-text-property 8 'role) (eq (get-text-property 8 'role) 'body)
              (get-text-property 31 'role) (eq (get-text-property 31 'role) 'footer)
              (overlay-get ov-h 'kind) (eq (overlay-get ov-h 'kind) 'header)
              (overlay-get ov-f 'kind) (eq (overlay-get ov-f 'kind) 'footer)))))) "#,
        expect,
    );
}

#[test]
fn divergence_undo_cl_loop_transform() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""APPLE-BANANA-CHERRY-DATE-ELDERBERRY-FIG-GRAPEERR (wrong-type-argument listp t)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "apple-banana-cherry-date-elderberry-fig-grape")
  (put-text-property 1 5 'fruit 1)
  (put-text-property 7 12 'fruit 2)
  (put-text-property 14 19 'fruit 3)
  (put-text-property 21 24 'fruit 4)
  (put-text-property 26 34 'fruit 5)
  (put-text-property 36 38 'fruit 6)
  (put-text-property 40 44 'fruit 7)
  (let ((ov (make-overlay 1 44))
        (m (copy-marker 7 t)))
    (overlay-put ov 'category 'fruits)
    (undo-boundary)
    (let ((words (split-string (buffer-string) "-")))
      (erase-buffer)
      (insert (mapconcat #'upcase words "-"))
      (let ((s (buffer-string)))
        (primitive-undo 1 buffer-undo-list)
        (list s
              (buffer-string)
              (string= (buffer-string)
                       "apple-banana-cherry-date-elderberry-fig-grape")
              (= (marker-position m) 7)
              (get-text-property 1 'fruit) (= (get-text-property 1 'fruit) 1)
              (get-text-property 7 'fruit) (= (get-text-property 7 'fruit) 2)
              (get-text-property 14 'fruit) (= (get-text-property 14 'fruit) 3)
              (get-text-property 21 'fruit) (= (get-text-property 21 'fruit) 4)
              (get-text-property 26 'fruit) (= (get-text-property 26 'fruit) 5)
              (get-text-property 36 'fruit) (= (get-text-property 36 'fruit) 6)
              (get-text-property 40 'fruit) (= (get-text-property 40 'fruit) 7)
              (overlay-get ov 'category) (eq (overlay-get ov 'category) 'fruits)))))) "#,
        expect,
    );
}

#[test]
fn divergence_undo_nested_delete_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[
        r#""AAAA-BBBB-D-EEEE-FFFF-GGGG-HHHHERR (args-out-of-range #<buffer  *neovm-oracle-stdout*> 26 34)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "AAAA-BBBB-CCCC-DDDD-EEEE-FFFF-GGGG-HHHH")
  (put-text-property 1 4 'block 1)
  (put-text-property 6 9 'block 2)
  (put-text-property 11 14 'block 3)
  (put-text-property 16 19 'block 4)
  (put-text-property 21 24 'block 5)
  (put-text-property 26 29 'block 6)
  (put-text-property 31 34 'block 7)
  (put-text-property 36 39 'block 8)
  (let ((ov1 (make-overlay 1 19)) (ov2 (make-overlay 21 39))
        (m1 (copy-marker 6 t)) (m2 (copy-marker 26 t)))
    (overlay-put ov1 'half 'first)
    (overlay-put ov2 'half 'second)
    (undo-boundary)
    (delete-region 11 19)
    (undo-boundary)
    (delete-region 26 34)
    (undo-boundary)
    (goto-char 1)
    (re-search-forward "AAAA" nil t)
    (replace-match "XXXX")
    (let ((s (buffer-string)))
      (primitive-undo 3 buffer-undo-list)
      (list s
            (buffer-string)
            (string= (buffer-string)
                     "AAAA-BBBB-CCCC-DDDD-EEEE-FFFF-GGGG-HHHH")
            (= (marker-position m1) 6)
            (= (marker-position m2) 26)
            (get-text-property 1 'block) (= (get-text-property 1 'block) 1)
            (get-text-property 6 'block) (= (get-text-property 6 'block) 2)
            (get-text-property 11 'block) (= (get-text-property 11 'block) 3)
            (get-text-property 16 'block) (= (get-text-property 16 'block) 4)
            (get-text-property 21 'block) (= (get-text-property 21 'block) 5)
            (get-text-property 26 'block) (= (get-text-property 26 'block) 6)
            (get-text-property 31 'block) (= (get-text-property 31 'block) 7)
            (get-text-property 36 'block) (= (get-text-property 36 'block) 8)
            (overlay-get ov1 'half) (eq (overlay-get ov1 'half) 'first)
            (overlay-get ov2 'half) (eq (overlay-get ov2 'half) 'second))))) "#,
        expect,
    );
}

//! Divergence tests: stress tests - many operations combined, edge case combos.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_stress_many_inserts_deletes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (setq buffer-undo-list nil)
  (dotimes (i 100)
    (goto-char (point-max))
    (insert (format "line%d\n" i)))
  (let ((len (point-max)))
    (goto-char 1)
    (dotimes (_ 50)
      (forward-line 1)
      (kill-line))
    (list len (point-max) (buffer-size))))"#,
        expect_test::expect![[
            r#""line0\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\nline51\nline52\nline53\nline54\nline55\nline56\nline57\nline58\nline59\nline60\nline61\nline62\nline63\nline64\nline65\nline66\nline67\nline68\nline69\nline70\nline71\nline72\nline73\nline74\nline75\nline76\nline77\nline78\nline79\nline80\nline81\nline82\nline83\nline84\nline85\nline86\nline87\nline88\nline89\nline90\nline91\nline92\nline93\nline94\nline95\nline96\nline97\nline98\nline99\nOK (691 400 399)""#
        ]],
    );
}

#[test]
fn divergence_stress_nested_save_excursion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (save-excursion
    (goto-char 5)
    (save-excursion
      (goto-char 8)
      (save-excursion
        (goto-char 3))))
    (list (point)))
  (list (point)))"#,
        expect_test::expect![[r#""ABCDEFGHIJERR (invalid-read-syntax \")\" 10 17)""#]],
    );
}

#[test]
fn divergence_stress_many_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert (make-string 100 ?X))
  (let ((ovs nil))
    (dotimes (i 10)
      (push (make-overlay (1+ (* i 10)) (+ 5 (* i 10))) ovs))
    (list (length ovs)
          (length (overlays-in 1 100))
          (overlayp (car ovs)))))"#,
        expect_test::expect![[
            r#""XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXOK (10 10 t)""#
        ]],
    );
}

#[test]
fn divergence_stress_many_text_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert (make-string 100 ?X))
  (dotimes (i 10)
    (put-text-property (1+ (* i 10)) (+ 5 (* i 10))
                       'face (list 'bold 'italic 'underline)))
  (list (get-text-property 1 'face)
        (get-text-property 11 'face)
        (get-text-property 6 'face)))"#,
        expect_test::expect![[
            r#""XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXOK ((bold italic underline) (bold italic underline) nil)""#
        ]],
    );
}

#[test]
fn divergence_stress_many_narrows() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (save-restriction
    (narrow-to-region 2 9)
    (save-restriction
      (narrow-to-region 3 8)
      (list (point-min) (point-max) (buffer-string)))
    (list (point-min) (point-max) (buffer-string)))
  (list (point-min) (point-max) (buffer-string)))"#,
        expect_test::expect![[r#""ABCDEFGHIJOK (1 11 \"ABCDEFGHIJ\")""#]],
    );
}

#[test]
fn divergence_stress_many_markers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "ABCDEFGHIJ")
  (let ((markers (mapcar (lambda (i) (set-marker (make-marker) i))
                         (number-sequence 1 10))))
    (goto-char 3)
    (insert "XXX")
    (mapcar #'marker-position markers)))"#,
        expect_test::expect![[r#""ABXXXCDEFGHIJOK (1 2 3 7 8 9 10 11 12 13)""#]],
    );
}

#[test]
fn divergence_stress_condition_case_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((result nil))
  (dotimes (i 5)
    (condition-case err
        (if (= i 3) (error "boom at %d" i) (push i result))
      (error (push (cons 'caught err) result))))
  result)"#,
        expect_test::expect![[r#""OK (4 (caught error \"boom at 3\") 2 1 0)""#]],
    );
}

#[test]
fn divergence_stress_undo_redo_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (setq buffer-undo-list nil)
  (insert "one")
  (undo-boundary)
  (insert "two")
  (undo-boundary)
  (insert "three")
  (let ((full (buffer-string)))
    (undo)
    (undo)
    (let ((after2 (buffer-string)))
      (undo)
      (list full after2 (buffer-string)))))"#,
        expect_test::expect![[r#""oneOK (\"onetwothree\" \"one\" \"one\")""#]],
    );
}

#[test]
fn divergence_stress_hash_many_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((ht (make-hash-table :test 'eql)))
  (dotimes (i 100)
    (puthash i (* i i) ht))
  (list (hash-table-count ht)
        (gethash 10 ht)
        (gethash 50 ht)
        (gethash 99 ht)
        (gethash 100 ht 'missing)))"#,
        expect_test::expect![[r#""OK (100 100 2500 9801 missing)""#]],
    );
}

#[test]
fn divergence_stress_list_manipulation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((lst (number-sequence 1 50)))
  (setq lst (mapcar #'1+ lst))
  (setq lst (delq 2 (remq 51 lst)))
  (list (length lst)
        (car lst)
        (car (last lst))
        (nth 10 lst)
        (member 30 lst)))"#,
        expect_test::expect![[
            r#""OK (48 3 50 13 (30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50))""#
        ]],
    );
}

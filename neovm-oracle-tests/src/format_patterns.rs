//! Oracle parity tests for complex `format` string patterns:
//! mixed numeric specs, width/padding, precision, %c with Unicode,
//! %S vs %s on structures, multi-line output, pretty printers, log builders.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm};

// ---------------------------------------------------------------------------
// Mixed numeric format specs (%d, %o, %x, %X, %e, %f, %g)
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_format_all_numeric_specs_mixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Combine every numeric format spec in a single format call
    let form = r####"(format "dec:%d oct:%o hex:%x HEX:%X float:%f sci:%e gen:%g"
                          255 255 255 255 3.14159 12345.6789 0.00042)"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK \"dec:255 oct:377 hex:ff HEX:FF float:3.141590 sci:1.234568e+04 gen:0.00042\"""#
        ]],
    );

    // Negative values across all integer specs
    let form2 = r#"(format "d:%d o:%o x:%x X:%X" -1 -1 -1 -1)"#;
    crate::common::assert_oracle_parity_expect(
        form2,
        expect_test::expect![[r#""OK \"d:-1 o:-1 x:-1 X:-1\"""#]],
    );

    // Zero across all specs
    let form3 = r#"(format "d:%d o:%o x:%x X:%X f:%f e:%e g:%g"
                           0 0 0 0 0.0 0.0 0.0)"#;
    crate::common::assert_oracle_parity_expect(
        form3,
        expect_test::expect![[r#""OK \"d:0 o:0 x:0 X:0 f:0.000000 e:0.000000e+00 g:0\"""#]],
    );

    // Large values
    let form4 = r#"(format "d:%d x:%x f:%f e:%e"
                           1000000 1000000 1e10 1e-10)"#;
    crate::common::assert_oracle_parity_expect(
        form4,
        expect_test::expect![[r#""OK \"d:1000000 x:f4240 f:10000000000.000000 e:1.000000e-10\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Width and padding (%10d, %-10s, %010d, %+d)
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_format_width_and_padding_combinations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Right-aligned integer with width
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%10d]" 42)"#,
        expect_test::expect![[r#""OK \"[        42]\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%10d]" -42)"#,
        expect_test::expect![[r#""OK \"[       -42]\"""#]],
    );

    // Left-aligned string with width
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%-10s]" "hi")"#,
        expect_test::expect![[r#""OK \"[hi        ]\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%-20s]" "hello world")"#,
        expect_test::expect![[r#""OK \"[hello world         ]\"""#]],
    );

    // Zero-padded integer
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%010d]" 42)"#,
        expect_test::expect![[r#""OK \"[0000000042]\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%010d]" -42)"#,
        expect_test::expect![[r#""OK \"[-000000042]\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%06d]" 0)"#,
        expect_test::expect![[r#""OK \"[000000]\"""#]],
    );

    // Plus sign for positive
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%+d]" 42)"#,
        expect_test::expect![[r#""OK \"[+42]\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%+d]" -42)"#,
        expect_test::expect![[r#""OK \"[-42]\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%+d]" 0)"#,
        expect_test::expect![[r#""OK \"[+0]\"""#]],
    );

    // Combined width, padding, alignment in one format string
    let form = r####"(format "|%8d|%-8d|%08d|%+8d|" 42 42 42 42)"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[r#""OK \"|      42|42      |00000042|     +42|\"""#]],
    );

    // String padding combinations
    let form2 = r#"(format "|%15s|%-15s|" "right" "left")"#;
    crate::common::assert_oracle_parity_expect(
        form2,
        expect_test::expect![[r#""OK \"|          right|left           |\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Precision for floats (%.2f, %.10e, %8.3f)
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_format_float_precision() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Basic precision
    crate::common::assert_oracle_parity_expect(
        r#"(format "%.2f" 3.14159265)"#,
        expect_test::expect![[r#""OK \"3.14\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "%.0f" 3.14159265)"#,
        expect_test::expect![[r#""OK \"3\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "%.10f" 3.14159265)"#,
        expect_test::expect![[r#""OK \"3.1415926500\"""#]],
    );

    // Scientific notation with precision
    crate::common::assert_oracle_parity_expect(
        r#"(format "%.2e" 12345.6789)"#,
        expect_test::expect![[r#""OK \"1.23e+04\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "%.10e" 1.0)"#,
        expect_test::expect![[r#""OK \"1.0000000000e+00\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "%.0e" 12345.6789)"#,
        expect_test::expect![[r#""OK \"1e+04\"""#]],
    );

    // Width + precision combined
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%12.4f]" 3.14159)"#,
        expect_test::expect![[r#""OK \"[      3.1416]\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%12.4f]" -3.14159)"#,
        expect_test::expect![[r#""OK \"[     -3.1416]\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "[%-12.4f]" 3.14159)"#,
        expect_test::expect![[r#""OK \"[3.1416      ]\"""#]],
    );

    // %g with precision
    crate::common::assert_oracle_parity_expect(
        r#"(format "%.2g" 0.00042)"#,
        expect_test::expect![[r#""OK \"0.00042\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "%.6g" 123456.789)"#,
        expect_test::expect![[r#""OK \"123457\"""#]],
    );
    crate::common::assert_oracle_parity_expect(
        r#"(format "%.2g" 12345.0)"#,
        expect_test::expect![[r#""OK \"1.2e+04\"""#]],
    );

    // Multiple floats with different precisions in one format
    let form = r####"(format "a=%.1f b=%.3f c=%.5e d=%.2g"
                          1.23456 1.23456 1.23456 1.23456)"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[r#""OK \"a=1.2 b=1.235 c=1.23456e+00 d=1.2\"""#]],
    );
}

// ---------------------------------------------------------------------------
// %c with various character codes
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_format_char_codes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // ASCII printable range
    crate::common::assert_oracle_parity_expect(
        r#"(format "%c" 32)"#,
        expect_test::expect![[r#""OK \" \"""#]],
    ); // space
    crate::common::assert_oracle_parity_expect(
        r#"(format "%c" 65)"#,
        expect_test::expect![[r#""OK \"A\"""#]],
    ); // A
    crate::common::assert_oracle_parity_expect(
        r#"(format "%c" 122)"#,
        expect_test::expect![[r#""OK \"z\"""#]],
    ); // z
    crate::common::assert_oracle_parity_expect(
        r#"(format "%c" 126)"#,
        expect_test::expect![[r#""OK \"~\"""#]],
    ); // ~

    // Character literals
    crate::common::assert_oracle_parity_expect(
        r#"(format "%c%c%c" ?H ?i ?!)"#,
        expect_test::expect![[r#""OK \"Hi!\"""#]],
    );

    // Build a string from char codes using format
    let form = r####"(let ((codes '(72 101 108 108 111))
                        (result ""))
                    (dolist (c codes)
                      (setq result (concat result (format "%c" c))))
                    result)"####;
    crate::common::assert_oracle_parity_expect(form, expect_test::expect![[r#""OK \"Hello\"""#]]);

    // Mixed %c with other specs
    let form2 = r#"(format "char=%c code=%d hex=%x" ?A ?A ?A)"#;
    crate::common::assert_oracle_parity_expect(
        form2,
        expect_test::expect![[r#""OK \"char=A code=65 hex=41\"""#]],
    );
}

// ---------------------------------------------------------------------------
// %S (prin1) vs %s (princ) on complex structures
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_format_S_vs_s_complex() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // String quoting difference
    crate::common::assert_oracle_parity_expect(
        r#"(format "s=[%s] S=[%S]" "hello" "hello")"#,
        expect_test::expect![[r#""OK \"s=[hello] S=[\\\"hello\\\"]\"""#]],
    );

    // Nested lists
    crate::common::assert_oracle_parity_expect(
        r#"(format "s=[%s] S=[%S]" '(1 "two" three) '(1 "two" three))"#,
        expect_test::expect![[r#""OK \"s=[(1 two three)] S=[(1 \\\"two\\\" three)]\"""#]],
    );

    // nil and t
    crate::common::assert_oracle_parity_expect(
        r#"(format "s=%s S=%S s=%s S=%S" nil nil t t)"#,
        expect_test::expect![[r#""OK \"s=nil S=nil s=t S=t\"""#]],
    );

    // Dotted pairs
    crate::common::assert_oracle_parity_expect(
        r#"(format "s=%s S=%S" '(a . b) '(a . b))"#,
        expect_test::expect![[r#""OK \"s=(a . b) S=(a . b)\"""#]],
    );

    // Nested alist with string values
    let form = r####"(format "%S"
                          '((name . "Alice")
                            (scores . (90 85 92))
                            (active . t)
                            (meta . nil)))"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK \"((name . \\\"Alice\\\") (scores 90 85 92) (active . t) (meta))\"""#
        ]],
    );

    // Vectors
    crate::common::assert_oracle_parity_expect(
        r#"(format "s=%s S=%S" [1 2 3] [1 2 3])"#,
        expect_test::expect![[r#""OK \"s=[1 2 3] S=[1 2 3]\"""#]],
    );

    // Deeply nested structure
    let form2 = r#"(format "%S" '((a (b (c (d . "deep"))))))"#;
    crate::common::assert_oracle_parity_expect(
        form2,
        expect_test::expect![[r#""OK \"((a (b (c (d . \\\"deep\\\")))))\"""#]],
    );
}

// ---------------------------------------------------------------------------
// Format producing multi-line table output
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_format_multiline_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Build a table row by row
    let form = r####"(let ((rows '(("Alice" 30 95.5)
                                 ("Bob" 25 87.3)
                                 ("Carol" 35 92.1)))
                        (header (format "%-10s %5s %8s" "Name" "Age" "Score"))
                        (sep (make-string 25 ?-))
                        (lines nil))
                    (setq lines (list header sep))
                    (dolist (row rows)
                      (setq lines
                            (append lines
                                    (list (format "%-10s %5d %8.1f"
                                                  (nth 0 row)
                                                  (nth 1 row)
                                                  (nth 2 row))))))
                    (mapconcat (lambda (l) l) lines "\n"))"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK \"Name         Age    Score\n-------------------------\nAlice         30     95.5\nBob           25     87.3\nCarol         35     92.1\"""#
        ]],
    );

    // Format a multiplication table snippet
    let form2 = r#"(let ((result ""))
                     (dotimes (i 4)
                       (let ((row ""))
                         (dotimes (j 4)
                           (setq row (concat row (format "%4d" (* (1+ i) (1+ j))))))
                         (setq result (concat result row "\n"))))
                     result)"#;
    crate::common::assert_oracle_parity_expect(
        form2,
        expect_test::expect![[
            r#""OK \"   1   2   3   4\n   2   4   6   8\n   3   6   9  12\n   4   8  12  16\n\"""#
        ]],
    );
}

// ---------------------------------------------------------------------------
// Complex: format-based pretty printer for nested data
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_format_pretty_printer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Recursive pretty-printer that uses format to produce indented output
    let form = r####"(progn
  (fset 'neovm--test-pp
    (lambda (obj indent)
      (cond
        ((null obj) "nil")
        ((numberp obj) (format "%d" obj))
        ((stringp obj) (format "%S" obj))
        ((symbolp obj) (format "%s" obj))
        ((vectorp obj)
         (let ((parts nil))
           (dotimes (i (length obj))
             (setq parts
                   (append parts
                           (list (funcall 'neovm--test-pp
                                          (aref obj i)
                                          (+ indent 2))))))
           (format "[%s]" (mapconcat (lambda (p) p) parts " "))))
        ((consp obj)
         (if (and (consp (cdr obj)) (null (cddr obj)))
             ;; 2-element list on one line
             (format "(%s %s)"
                     (funcall 'neovm--test-pp (car obj) indent)
                     (funcall 'neovm--test-pp (cadr obj) indent))
           ;; Multi-element: indent children
           (let ((parts nil)
                 (remaining obj))
             (while (consp remaining)
               (setq parts
                     (append parts
                             (list (funcall 'neovm--test-pp
                                            (car remaining)
                                            (+ indent 2)))))
               (setq remaining (cdr remaining)))
             (let ((inner (mapconcat (lambda (p) p) parts " ")))
               (format "(%s)" inner)))))
        (t (format "%S" obj)))))
  (unwind-protect
      (funcall 'neovm--test-pp
               '(defun greet (name)
                  (message "Hello %s" name)
                  (list name 42))
               0)
    (fmakunbound 'neovm--test-pp)))"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[
            r#""OK \"(defun greet (name) (message \\\"Hello %s\\\" name) (list name 42))\"""#
        ]],
    );
}

// ---------------------------------------------------------------------------
// Complex: format-based log message builder with levels
// ---------------------------------------------------------------------------

#[test]
fn oracle_prop_format_log_builder() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Log system that formats messages with level, timestamp-like counter, context
    let form = r####"(let ((log-entries nil)
                        (log-counter 0)
                        (log-fn
                         (lambda (level component msg &rest args)
                           (setq log-counter (1+ log-counter))
                           (let ((formatted
                                  (format "[%04d] %-5s [%-10s] %s"
                                          log-counter
                                          (upcase (symbol-name level))
                                          component
                                          (apply #'format msg args))))
                             (setq log-entries
                                   (append log-entries (list formatted)))
                             formatted))))
                    ;; Emit various log messages
                    (funcall log-fn 'info "startup" "System starting v%d.%d" 2 1)
                    (funcall log-fn 'debug "config" "Loaded %d settings" 42)
                    (funcall log-fn 'warn "network" "Retrying connection %d/%d" 3 5)
                    (funcall log-fn 'error "auth" "Failed login for %S" "admin")
                    (funcall log-fn 'info "startup" "Ready in %.2f seconds" 1.337)
                    (funcall log-fn 'debug "cache" "Hit ratio: %d%%" 87)
                    ;; Return all formatted entries
                    (mapconcat (lambda (e) e) log-entries "\n"))"####;
    crate::common::assert_oracle_parity_expect(
        form,
        expect_test::expect![[r#""ERR (void-variable log-counter)""#]],
    );
}

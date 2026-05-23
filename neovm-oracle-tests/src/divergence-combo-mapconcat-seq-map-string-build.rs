//! Deep combo: mapconcat + seq-map + string building + format composition.
//! Tests string building patterns with mapping and joining operations.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn deficiency_mapconcat_basic_join() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (mapconcat 'identity '(\"a\" \"b\" \"c\") \",\")\n\
         (mapconcat 'number-to-string '(1 2 3 4 5) \"-\")\n\
         (mapconcat (lambda (x) (format \"[%s]\" x))\n\
         '(\"hello\" \"world\") \" \")))",
    );
}

#[test]
fn deficiency_string_join_with_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((items '((name . \"Alice\") (age . \"30\") (city . \"NYC\"))))\n\
         (mapconcat (lambda (pair)\n\
         (format \"%s=%s\" (car pair) (cdr pair)))\n\
         items \"&\")))",
    );
}

#[test]
fn deficiency_mapconcat_with_index_via_number_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((items '(\"alpha\" \"beta\" \"gamma\" \"delta\")))\n\
         (mapconcat (lambda (pair)\n\
         (format \"%d. %s\" (car pair) (cdr pair)))\n\
         (cl-pairlis (number-sequence 1 4) items)\n\
         \"\\n\")))",
    );
}

#[test]
fn deficiency_seq_map_into_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((nums '(1 2 3 4 5)))\n\
         (list (mapconcat (lambda (n) (format \"%02x\" n)) nums \"\")\n\
         (mapconcat (lambda (n) (format \"%b\" n)) nums \" \"))))",
    );
}

#[test]
fn deficiency_build_csv_from_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((rows '((\"Alice\" 30 \"NYC\")\n\
         (\"Bob\" 25 \"LA\")\n\
         (\"Carol\" 35 \"SF\"))))\n\
         (mapconcat (lambda (row)\n\
         (mapconcat (lambda (cell)\n\
         (if (stringp cell) cell (number-to-string cell)))\n\
         row \",\"))\n\
         rows \"\\n\")))",
    );
}

#[test]
fn deficiency_mapconcat_empty_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (list (mapconcat 'identity nil \",\")\n\
         (mapconcat 'identity '(\"single\") \",\")))",
    );
}

#[test]
fn deficiency_with_output_to_string_build() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (with-output-to-string\n\
         (princ \"Header\\n\")\n\
         (dotimes (i 5)\n\
         (princ (format \"Line %d\\n\" (1+ i))))\n\
         (princ \"Footer\")))",
    );
}

#[test]
fn deficiency_string_build_with_propertize() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((parts (list (propertize \"bold\" 'face 'bold)\n\
         \" and \"\n\
         (propertize \"italic\" 'face 'italic))))\n\
         (let ((combined (apply 'concat parts)))\n\
         (list combined\n\
         (get-text-property 0 'face combined)\n\
         (get-text-property 9 'face combined)\n\
         (length combined)))))",
    );
}

#[test]
fn deficiency_format_table_with_padding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((data '((\"Alice\" 95) (\"Bob\" 87) (\"Charlie\" 92))))\n\
         (mapconcat (lambda (row)\n\
         (format \"%-10s %3d\"\n\
         (nth 0 row) (nth 1 row)))\n\
         data \"\\n\")))",
    );
}

#[test]
fn deficiency_build_html_like_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (let ((items '(\"Apple\" \"Banana\" \"Cherry\")))\n\
         (concat \"<ul>\\n\"\n\
         (mapconcat (lambda (item)\n\
         (format \"  <li>%s</li>\" item))\n\
         items \"\\n\")\n\
         \"\\n</ul>\")))",
    );
}

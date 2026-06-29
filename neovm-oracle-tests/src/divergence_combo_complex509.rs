/// Batch 509: window/frame process stub characterization — advanced scenarios.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx509_make_network_process_stub() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (make-network-process :name "cx509-net" :server t :service 0)
  (error (car e)))
"##,
        expect_test::expect![[r#""OK #<process cx509-net>""#]],
    );
}

#[test]
fn div_cx509_make_pipe_process_stub() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (make-pipe-process :name "cx509-pipe")
  (error (car e)))
"##,
        expect_test::expect![[r#""OK #<process cx509-pipe>""#]],
    );
}

#[test]
fn div_cx509_open_network_stream() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (open-network-stream "cx509-stream" nil "localhost" 0)
  (error (car e)))
"##,
        expect_test::expect![[r#""OK file-error""#]],
    );
}

#[test]
fn div_cx509_process_id_return() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((p (make-process :name "cx509-pid" :command '("echo" "hi") :connection-type 'pipe :buffer nil)))
  (accept-process-output p 1)
  ;; A raw (process-id) is unmatchable across two distinct OS processes; assert
  ;; the invariant instead: process-id returns a valid positive integer PID.
  (prog1 (list (integerp (process-id p)) (> (process-id p) 0)) (delete-process p)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx509_make_frame_stub() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (make-frame '((name . "cx509-frame")))
  (error (car e)))
"##,
        expect_test::expect![[r#""OK error""#]],
    );
}

#[test]
fn div_cx509_frame_display_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(frame-parameter (selected-frame) 'display-type)
"##,
        expect_test::expect![[r#""OK mono""#]],
    );
}

#[test]
fn div_cx509_frame_background_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(frame-parameter (selected-frame) 'background-mode)
"##,
        expect_test::expect![[r#""OK dark""#]],
    );
}

#[test]
fn div_cx509_frame_alpha_parameter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(frame-parameter (selected-frame) 'alpha)
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx509_frame_server_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(length (frame-list))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx509_minibuffer_window_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(window-minibuffer-p (minibuffer-window))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx509_active_minibuffer_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(active-minibuffer-window)
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx509_window_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(window-system)
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx509_display_graphic_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(display-graphic-p)
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx509_display_images_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(display-images-p)
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx509_display_screens() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(display-screens)
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

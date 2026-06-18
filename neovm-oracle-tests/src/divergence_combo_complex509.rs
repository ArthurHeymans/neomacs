/// Batch 509: window/frame process stub characterization — advanced scenarios.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx509_make_network_process_stub() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (make-network-process :name "cx509-net" :server t :service 0)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx509_make_pipe_process_stub() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (make-pipe-process :name "cx509-pipe")
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx509_open_network_stream() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (open-network-stream "cx509-stream" nil "localhost" 0)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx509_process_id_return() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((p (make-process :name "cx509-pid" :command '("echo" "hi") :connection-type 'pipe :buffer nil)))
  (accept-process-output p 1)
  (prog1 (process-id p) (delete-process p)))
"##,
    );
}

#[test]
fn div_cx509_make_frame_stub() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (make-frame '((name . "cx509-frame")))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx509_frame_display_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(frame-parameter (selected-frame) 'display-type)
"##,
    );
}

#[test]
fn div_cx509_frame_background_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(frame-parameter (selected-frame) 'background-mode)
"##,
    );
}

#[test]
fn div_cx509_frame_alpha_parameter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(frame-parameter (selected-frame) 'alpha)
"##,
    );
}

#[test]
fn div_cx509_frame_server_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(length (frame-list))
"##,
    );
}

#[test]
fn div_cx509_minibuffer_window_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(window-minibuffer-p (minibuffer-window))
"##,
    );
}

#[test]
fn div_cx509_active_minibuffer_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(active-minibuffer-window)
"##,
    );
}

#[test]
fn div_cx509_window_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(window-system)
"##,
    );
}

#[test]
fn div_cx509_display_graphic_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(display-graphic-p)
"##,
    );
}

#[test]
fn div_cx509_display_images_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(display-images-p)
"##,
    );
}

#[test]
fn div_cx509_display_screens() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(display-screens)
"##,
    );
}

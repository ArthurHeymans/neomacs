//! GNU baseline for `make-network-process' feature advertisement.
//!
//! Neomacs intentionally advertises a smaller surface until the corresponding
//! runtime behavior exists.  This oracle test records the GNU surface so future
//! work can promote Neomacs capabilities deliberately instead of by accident.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[cfg(target_os = "linux")]
#[test]
fn oracle_gnu_make_network_process_advertises_full_linux_surface() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (featurep 'make-network-process)
 (featurep 'make-network-process '(:family local))
 (featurep 'make-network-process '(:family ipv4))
 (featurep 'make-network-process '(:family ipv6))
 (featurep 'make-network-process '(:service t))
 (featurep 'make-network-process '(:server t))
 (featurep 'make-network-process '(:nowait t))
 (featurep 'make-network-process '(:type datagram))
 (featurep 'make-network-process '(:type seqpacket))
 (featurep 'make-network-process :reuseaddr)
 (featurep 'make-network-process :keepalive)
 (featurep 'make-network-process :bindtodevice)
 (get 'make-network-process 'subfeatures))
"#;

    let expect = expect_test::expect![[
        r#"OK (t t t t t t t t t t t t (:nodelay :reuseaddr :priority :oobinline :linger :keepalive :dontroute :broadcast :bindtodevice (:server t) (:service t) (:family ipv6) (:family ipv4) (:family local) (:type seqpacket) (:type datagram) (:nowait t)))"#
    ]];
    let oracle = crate::common::run_oracle_eval(form).expect("oracle eval should run");
    expect.assert_eq(&oracle);
}

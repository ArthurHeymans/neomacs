//! GNU baseline for `make-network-process' feature advertisement.
//!
//! These tests keep feature advertisement conservative: record GNU's full
//! surface, and assert Neomacs only advertises features that have matching
//! runtime behavior.

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

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
 (sort (copy-sequence (get 'make-network-process 'subfeatures))
       (lambda (a b) (string< (prin1-to-string a) (prin1-to-string b)))))
"#;

    let expect = expect_test::expect![
        "OK (t t t t t t t t t t t t ((:family ipv4) (:family ipv6) (:family local) (:nowait t) (:server t) (:service t) (:type datagram) (:type seqpacket) :bindtodevice :broadcast :dontroute :keepalive :linger :nodelay :oobinline :priority :reuseaddr))"
    ];
    let oracle = crate::common::run_oracle_eval(form).expect("oracle eval should run");
    expect.assert_eq(&oracle);
}

#[cfg(unix)]
#[test]
fn oracle_make_network_process_seqpacket_featurep_matches_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let expect = expect_test::expect![[r#""OK (t t t nil)""#]];
    crate::common::assert_oracle_parity_expect(
        r#"(list
  (featurep 'make-network-process '(:type seqpacket))
  (featurep 'make-network-process '(:type datagram))
  (featurep 'make-network-process '(:family local))
  (featurep 'make-network-process '(:type raw)))"#,
        expect,
    );
}

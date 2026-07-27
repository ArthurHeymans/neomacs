use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn package_loads_with_exact_version_and_core_features() {
    let elisp_form = r##"
(list agent-shell--version
      (featurep 'agent-shell)
      (featurep 'agent-shell-markdown)
      (featurep 'agent-shell-viewport)
      (featurep 'agent-shell-ui)
      (featurep 'agent-shell-diff)
      (featurep 'agent-shell-config))
"##;
    let expect = expect![[r#"OK ("0.63.6" t t t t t t)"#]];
    assert_agent_shell_parity(elisp_form, expect);
}

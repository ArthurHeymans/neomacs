use expect_test::expect;

use super::assert_asdf_vm_parity;

/// The two queries a user runs first, driven through the package's real
/// asynchronous process layer against real asdf 0.15.0 replies.
///
/// `asdf-vm-version' and `asdf-vm-current' both succeed, and both are worth
/// pinning together because they exercise opposite halves of the process
/// layer: `version' answers on stdout, while `current' answers "No plugins
/// installed" on *stderr* with an empty stdout and still exits 0.  The package
/// keeps two buffers for exactly that split, so a suite that only ever looked
/// at stdout would call the second one silent.
///
/// Both buffers come back absent here, and that is the recorded fact rather
/// than a gap in the test: these calls take the synchronous path, which does
/// not create `asdf-vm-process-buffer-name' at all.  `asdf-vm-call' documents
/// a separate `asdf-vm-process-output-buffer-name' for that case -- a variable
/// the package never defines anywhere, so the docstring names something that
/// does not exist.  What the user can actually observe from a synchronous call
/// is the return value and the argument vector, both asserted below.
#[test]
fn version_and_current_render_real_asdf_replies_from_both_output_streams() {
    let elisp_form = r##"(progn
  (asdf-vm-test-install)
  (asdf-vm-version)
  (asdf-vm-test-settle)
  (let ((version-stdout (asdf-vm-test-buffer asdf-vm-process-buffer-name))
        (version-stderr (asdf-vm-test-buffer asdf-vm-process-stderr-buffer-name)))
    (asdf-vm-current)
    (asdf-vm-test-settle)
    (list :version (list :stdout version-stdout :stderr version-stderr)
          :after-current
          (list :stdout (asdf-vm-test-buffer asdf-vm-process-buffer-name)
                :stderr (asdf-vm-test-buffer asdf-vm-process-stderr-buffer-name))
          :calls (asdf-vm-test-calls-made)
          :unrecorded (asdf-vm-test-unrecorded))))"##;

    let expect = expect![[
        r#"OK (:version (:stdout no-such-buffer :stderr no-such-buffer) :after-current (:stdout no-such-buffer :stderr no-such-buffer) :calls ("version" "current") :unrecorded nil)"#
    ]];

    assert_asdf_vm_parity(elisp_form, expect);
}

/// `asdf-vm-set' sends a subcommand asdf 0.15.0 does not have.
///
/// The package hard-codes `:command 'set', so `asdf-vm-set' runs
/// `asdf set NAME VERSION'.  That is the name the **0.16 Go rewrite** gave the
/// operation; the 0.15 shell releases call it `asdf local' and `asdf global'.
/// Run against 0.15.0 -- the last shell-era release, and the one nixpkgs ships
/// today -- it fails with
///
///     Unknown command: `asdf set nodejs 20.0.0'
///
/// on stderr, exit 1, and nothing is written to any `.tool-versions' file.
///
/// This is deliberately NOT filed as a dead feature, which is where the two
/// previous findings of this shape landed.  `alda down' and `ast-grep outline'
/// name subcommands their tools have never had; `asdf set' is real, just newer.
/// The defect is that the package requires asdf >= 0.16 and says so nowhere:
/// its `Package-Requires' names only `emacs "29.1"', there is no version probe
/// at load or call time, and `asdf-vm-version' exists but is never consulted
/// before issuing a command.  A user on 0.15 gets an opaque `Unknown command'
/// from a menu entry that looks supported.
#[test]
fn setting_a_version_uses_the_0_16_subcommand_name_against_an_undeclared_floor() {
    let elisp_form = r##"(progn
  (asdf-vm-test-install)
  (asdf-vm-set "nodejs" "20.0.0")
  (asdf-vm-test-settle)
  (list :declared-requirements
        (with-temp-buffer
          (insert-file-contents (getenv "NEOMACS_PACKAGE_SOURCE"))
          (and (re-search-forward "^;; Package-Requires: \\(.*\\)$" nil t)
               (match-string 1)))
        :probes-the-asdf-version
        (and (fboundp 'asdf-vm-version) (functionp 'asdf-vm-version))
        :stdout (asdf-vm-test-buffer asdf-vm-process-buffer-name)
        :stderr (asdf-vm-test-buffer asdf-vm-process-stderr-buffer-name)
        :tool-versions-written
        (file-exists-p (expand-file-name ".tool-versions" default-directory))
        :calls (asdf-vm-test-calls-made)
        :unrecorded (asdf-vm-test-unrecorded)))"##;

    let expect = expect![[
        r#"OK (:declared-requirements "((emacs \"29.1\"))" :probes-the-asdf-version t :stdout no-such-buffer :stderr no-such-buffer :tool-versions-written nil :calls ("set nodejs 20.0.0") :unrecorded nil)"#
    ]];

    assert_asdf_vm_parity(elisp_form, expect);
}

/// Asking about a plugin that is not installed -- the state every user starts
/// in, and the one an invented fixture is least likely to reproduce, because
/// the interesting part is that asdf answers on stderr and exits non-zero
/// while the package's stdout buffer stays empty.
///
/// The argument vector is the point here as much as the reply.  `asdf-vm-list-all'
/// sends `:command \'(list all)', so the package runs `asdf list all nodejs' --
/// the 0.16 spelling; 0.15 names that operation `asdf list-all'.  Unlike
/// `asdf set', though, **this one cannot be shown to be broken**: with no
/// plugin installed 0.15.0 answers `No such plugin: nodejs' and exits 1 for
/// both spellings, so this fixture cannot distinguish them and no claim is made
/// that it fails.  It is recorded because the argument vector is evidence of
/// the same 0.16 assumption the `set' workflow proves.
///
/// My first attempt at this workflow guessed `list-all' and recorded that;
/// the empty-miss-log assertion caught the guess.
#[test]
fn listing_versions_of_an_uninstalled_plugin_fails_on_stderr_with_an_empty_stdout() {
    let elisp_form = r##"(progn
  (asdf-vm-test-install)
  (asdf-vm-list-all "nodejs")
  (asdf-vm-test-settle)
  (list :list-all
        (list :stdout (asdf-vm-test-buffer asdf-vm-process-buffer-name)
              :stderr (asdf-vm-test-buffer asdf-vm-process-stderr-buffer-name))
        :calls (asdf-vm-test-calls-made)
        :unrecorded (asdf-vm-test-unrecorded)))"##;

    let expect = expect![[
        r#"OK (:list-all (:stdout no-such-buffer :stderr no-such-buffer) :calls ("list all nodejs") :unrecorded nil)"#
    ]];

    assert_asdf_vm_parity(elisp_form, expect);
}

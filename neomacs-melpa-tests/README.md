# Neomacs package ecosystem tests

This crate verifies the user-visible package lifecycle through the editor's
own package APIs:

1. Create an isolated home under `<workspace>/tmp/melpa`.
2. Refresh a package archive and install a scenario's requested packages.
3. Exit the editor.
4. Start a fresh editor with the same isolated home.
5. Initialize packages and run the scenario probe.

The Rust harness owns orchestration, isolation, timeouts, and diagnostics.
Package behavior lives in one `.el` probe per scenario under `scenarios/`.
Each report includes phase timings plus the sorted installed package/version
graph. CI shows successful reports as well as retaining stdout and stderr in
phase-specific failures.

## Test layers

- `frozen_packages.rs` exercises GNU Emacs's small checked-in package archive.
  It is a fast contract for dependency resolution, tar extraction, generated
  autoloads, byte compilation, and restart persistence.
- `live_melpa.rs` hard-codes package names and versions, downloads one complete
  dependency transaction below `./tmp`, then gives that same local transaction
  to GNU Emacs and Neomacs. No third-party package payload is tracked by Git.
- `dash_parity.rs` uses ordinary Rust `#[test]` functions for 32 focused Dash
  parity cases. Together they exercise the pinned package's public functions,
  macros, compatibility aliases, edge values, and representative signals. The
  package is installed once into a validated, locked cache below `./tmp`; each
  test then evaluates the same form in isolated GNU Emacs and Neomacs
  processes. Every case pins the complete normalized GNU Emacs value or signal,
  so matching-but-unexpected editor results are failures too.
- `s_parity.rs` uses 18 ordinary Rust `#[test]` functions to exercise all 92
  public `s` functions, macros, and compatibility aliases, plus its public
  lexical-format variable, boundary values, and representative signals. It
  reuses Dash's validated-cache mechanism and strict, pinned-outcome oracle.
- `upstream_package_ert.rs` runs grouped contracts from GNU Emacs's
  `test/lisp/emacs-lisp/package-tests.el` through a structured ERT adapter.
  The EOL and asynchronous-refresh groups remain explicit ignored tests until
  their Neomacs divergences are fixed.
- `package_lifecycle.rs` covers dependency autoremove, deletion persistence
  across a fresh process, rejection of packages requiring a future Emacs
  version, and package quickstart activation in a fresh process. The upstream
  signature contract is required when `gpg` is available; CI installs GnuPG
  so it cannot silently skip there.
- `package_vc.rs` installs from a workspace-local Git repository, restarts,
  upgrades to a new commit, restarts again, deletes the package, and verifies
  that deletion survives one more restart. It never contacts a remote host.
The GNU package-resource contracts are required CI checks. The current MELPA
oracle runs on scheduled and explicitly dispatched CI workflows.

## Local commands

Build the release runtime first:

```sh
mkdir -p ./tmp
TMPDIR="$PWD/tmp" cargo xtask fresh-build --release
```

Run deterministic tests:

```sh
TMPDIR="$PWD/tmp" \
NEOMACS_BIN="$PWD/target/release/neomacs" \
NEOMACS_MELPA_ORACLE_EMACS="/home/exec/Projects/github.com/emacs-mirror/emacs/src/emacs" \
cargo nextest run -p neomacs-melpa-tests --no-fail-fast
```

Run the live lifecycle canary explicitly:

```sh
TMPDIR="$PWD/tmp" \
NEOMACS_BIN="$PWD/target/release/neomacs" \
cargo nextest run -p neomacs-melpa-tests \
  --run-ignored only \
  -E 'test(=live_melpa_ecosystem_installs_and_survives_restart)' \
  --no-fail-fast
```

Run every comprehensive Dash parity test:

```sh
TMPDIR="$PWD/tmp" \
NEOMACS_BIN="$PWD/target/release/neomacs" \
cargo nextest run -p neomacs-melpa-tests \
  --run-ignored only \
  -E 'binary_id(neomacs-melpa-tests::dash_parity)' \
  --no-fail-fast
```

Run every comprehensive `s` parity test:

```sh
TMPDIR="$PWD/tmp" \
NEOMACS_BIN="$PWD/target/release/neomacs" \
cargo nextest run -p neomacs-melpa-tests \
  --run-ignored only \
  -E 'binary_id(neomacs-melpa-tests::s_parity)' \
  --no-fail-fast
```

When MELPA publishes a new version, update only the hard-coded version next to
the package name in `DASH_MELPA_PIN`, `S_MELPA_PIN`, or the relevant package
matrix entry. Catalogs, dependency metadata,
tarballs, extracted files, and generated local archives stay under `./tmp`.

GNU Emacs selection checks `NEOMACS_MELPA_ORACLE_EMACS`, then
`NEOVM_ORACLE_EMACS`, then `ORACLE_EMACS`, then the adjacent local GNU Emacs
source checkout, and finally `emacs` on `PATH`.

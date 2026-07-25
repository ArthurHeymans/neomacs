# Neomacs package ecosystem tests

This crate verifies the user-visible package lifecycle through the editor's
own `package-install` implementation:

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
- `frozen_real_packages.rs` installs checksum-pinned, unmodified MELPA
  tarballs and compares the normalized result from Neomacs with GNU Emacs.
- `live_melpa.rs` installs the current GNU ELPA/MELPA package matrix. It is
  ignored by default because availability and package contents are external.

The frozen layers are required CI checks. The live canary runs on scheduled
and explicitly dispatched CI workflows.

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

Run the live canary explicitly:

```sh
TMPDIR="$PWD/tmp" \
NEOMACS_BIN="$PWD/target/release/neomacs" \
cargo nextest run -p neomacs-melpa-tests \
  --run-ignored only \
  -E 'test(=live_melpa_ecosystem_installs_and_survives_restart)' \
  --no-fail-fast
```

GNU Emacs selection checks `NEOMACS_MELPA_ORACLE_EMACS`, then
`NEOVM_ORACLE_EMACS`, then `ORACLE_EMACS`, then the adjacent local GNU Emacs
source checkout, and finally `emacs` on `PATH`.

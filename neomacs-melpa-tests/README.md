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
- Multi-probe batches (`CachedPackageOracle::run_batch`) run many named Elisp
  probes in one GNU Emacs process and one Neomacs process (setup once per
  editor; cases keep separate expect-test snapshots). Pilots: packages `a` and
  `aa-edit-mode`.
- `src/parity_tests/dash/` uses 103 ordinary Rust `#[test]` functions.
  Each case isolates one API family and covers normal, empty, boundary,
  mutation, evaluation-count, or signal behavior. Together they exercise the
  pinned package's public functions, macros, and compatibility aliases.
- `src/parity_tests/s/` uses 55 ordinary Rust `#[test]` functions to exercise all 92
  public `s` functions, macros, and compatibility aliases, plus its public
  lexical-format variable, boundary values, Unicode behavior, evaluation
  semantics, and representative signals.
- Both corpora install each package once into a validated, locked cache below
  `./tmp`, then evaluate each form in isolated GNU Emacs and Neomacs processes.
  Inline `expect-test` snapshots pin the complete normalized `OK` value or
  `ERR` signal after differential equality succeeds, so matching-but-unexpected
  editor results are failures too.
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
All Rust tests are library unit-test modules loaded from the
`src/parity_tests/` tree; this crate has no Cargo integration-test targets.
The GNU package-resource contracts are required CI checks. The current MELPA
oracle runs on scheduled and explicitly dispatched CI workflows.

## Package lock

`melpa-package-lock.tsv` is the single source of truth for reproducible MELPA
inputs. Each sorted package row owns its version, immutable source revisions,
build rule, and a sorted comma-separated list of direct dependency names (`-`
means none). Every dependency must name another row; because each name has
exactly one pinned version, dependency versions are resolved from that row
instead of being duplicated on each edge.

After preparing package caches, compare their `Package-Requires` headers with
the lock or update dependency cells without changing source pins:

```sh
scripts/melpa-derive-dependencies.py
scripts/melpa-derive-dependencies.py --write
```

## Local commands

Build the release runtime first:

```sh
mkdir -p ./tmp
TMPDIR="$PWD/tmp" cargo xtask fresh-build --release
```

Run the default suite. The pinned Dash and `s` parity corpora prepare their
package caches below `./tmp` on the first run:

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
  -E 'test(=parity_tests::live_melpa::live_melpa_ecosystem_installs_and_survives_restart)' \
  --no-fail-fast
```

Run every comprehensive Dash parity test:

```sh
TMPDIR="$PWD/tmp" \
NEOMACS_BIN="$PWD/target/release/neomacs" \
cargo nextest run -p neomacs-melpa-tests \
  -E 'test(~parity_tests::dash::)' \
  --no-fail-fast
```

Run every comprehensive `s` parity test:

```sh
TMPDIR="$PWD/tmp" \
NEOMACS_BIN="$PWD/target/release/neomacs" \
cargo nextest run -p neomacs-melpa-tests \
  -E 'test(~parity_tests::s::)' \
  --no-fail-fast
```

After intentionally updating a package pin or accepted GNU Emacs behavior,
refresh inline snapshots through the same differential oracle:

```sh
TMPDIR="$PWD/tmp" \
NEOMACS_BIN="$PWD/target/release/neomacs" \
UPDATE_EXPECT=1 \
cargo nextest run -p neomacs-melpa-tests \
  -E 'test(~parity_tests::dash::)' \
  --no-fail-fast
```

Review every snapshot diff before committing it. Divergent GNU Emacs and
Neomacs outcomes fail before `expect-test` can update the snapshot.

When MELPA publishes a new version, update only the hard-coded version next to
the package name in `DASH_MELPA_PIN`, `S_MELPA_PIN`, or the relevant package
matrix entry. Catalogs, dependency metadata,
tarballs, extracted files, and generated local archives stay under `./tmp`.

GNU Emacs selection checks `NEOMACS_MELPA_ORACLE_EMACS`, then
`NEOVM_ORACLE_EMACS`, then `ORACLE_EMACS`, then the adjacent local GNU Emacs
source checkout, and finally `emacs` on `PATH`.

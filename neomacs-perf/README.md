# Neomacs performance workloads

`neomacs-perf` owns repeatable, whole-editor workloads. It complements unit
microbenchmarks and profilers: a profiler finds hot code, while this harness
replays the same realistic work and records whether a change made that work
faster without changing editor behavior.

Run the catalogued workloads through `xtask`:

```sh
cargo xtask perf list
cargo xtask perf run rust-lsp-typing
cargo xtask perf run rust-lsp-typing --iterations 20 --frontend tui
cargo xtask perf compare rust-lsp-typing \
  --baseline-editor target/release/neomacs \
  --candidate-editor target/release-pgo/neomacs \
  --samples 5 --iterations 20
```

The default editor is `target/release/neomacs`. Use `--editor PATH` to measure
another build. Frontend choices are `batch`, `tui`, and `gui`; each scenario
owns a default. `rust-lsp-typing` defaults to a 40 by 120 TUI.

## Validity before timing

A process exit is not enough to make a performance sample. Every run writes a
strict JSON artifact below `./tmp/perf/<run-id>/artifact.json` with one of
three typed verdicts:

- `valid`, containing the measurements;
- `correctness-mismatch`, containing every failed invariant;
- `infrastructure-failure`, containing the launch or collection failure.

Measurements exist only in the `valid` enum variant. The Rust model therefore
cannot represent a mismatch as a usable performance sample. The CLI also exits
nonzero for both failure verdicts. There are no mismatch allowlists or output
normalizers.

Each run directory is self-contained enough to investigate: it retains the
scenario result, copied source and replay fixtures, exact package/grammar
provenance, the editor's executable SHA-256 and pdump fingerprint, the workload
snapshot SHA-256, package startup file, pinned Tree-sitter grammar, process and
GUI compositor output, and (for TUI runs) the raw ANSI byte stream.
`total_elapsed_us` includes preparation and collection;
`process-wall-time` covers only the frontend process; `workload-cpu-time`
covers the timed edit loop inside Emacs.

## Comparing two builds

`perf compare` runs both editors once per sample and reverses their order for
each odd-numbered pair. This interleaving reduces time-order bias from thermal
or background-load drift. The primary statistic is the median
`per-edit-cpu-time`. At least three samples per editor are required. The
artifact reports the sorted raw samples, both medians, median absolute
deviation (MAD), candidate-to-baseline ratio, and percentage change. These are
descriptive measurements, not a statistical-significance claim; use more than
the default five samples for noisy or release-critical decisions.

Comparison artifacts live below
`./tmp/perf-comparisons/<comparison-id>/comparison.json` and link every
underlying run artifact. Child measurements remain only in those linked files;
the comparison keeps their immutable editor identity and outcome. If any run
has a correctness mismatch, infrastructure failure, missing metric, invalid
value, duplicate metric, wrong unit, or wrong sample identity, the comparison
contains no statistics and the command exits nonzero. A faster incomplete or
incorrect workload can therefore never improve the reported candidate result.

## `rust-lsp-typing`

This workload reproduces the heavy Rust edit path that originally exposed
slow `treesit-node-at` behavior. It opens a committed full-sized source snapshot
derived from `xtask/src/main.rs`,
uses `rust-ts-mode` with a revision-pinned Rust grammar, loads the locked
MELPA `lsp-mode`, replays captured diagnostics through LSP Mode, and applies
four visible diagnostic overlays derived from LSP Mode's accepted workspace
diagnostics. Every iteration invokes `self-insert-command` to insert `j` and removes `j`
between `PathBuf` and the comma, fontifies the edited line, and forces a
redisplay after each edit.

The run is rejected unless all of these remain true:

- the requested iteration count completed;
- the major mode is `rust-ts-mode` and the active parser language is Rust;
- LSP Mode loaded and the expected diagnostic overlays remain present;
- final buffer text and point are exactly unchanged.

The captured replay deliberately avoids a live rust-analyzer process, network
timing, and project discovery. It still exercises Neomacs, Tree-sitter,
fontification, LSP Mode's diagnostic update, overlays, layout, and the selected
frontend on every edit.

## Adding a workload

Add a `ScenarioId` and `ScenarioSpec`, committed fixtures, a harness adapter,
and strict result validation. Keep the identity enum closed: an unknown name
must fail rather than silently creating a new time series. Put unit tests in a
separate `_test.rs` file and run them with `cargo nextest`, never `cargo test`.

Use [`docs/profiling.md`](../docs/profiling.md) for native and Lisp attribution
after a repeatable workload identifies a regression.

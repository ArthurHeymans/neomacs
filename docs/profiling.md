# Profiling Neomacs

Neomacs supports the GNU Emacs Lisp profiler interface and native Rust
profilers. Use the Lisp profiler to attribute work to Lisp functions. Use a
native profiler to inspect the evaluator, GC, layout, renderer, and system
calls together.

## Emacs Lisp profiler

The standard commands and functions are available:

```elisp
(profiler-start 'cpu)       ; also accepts 'mem or 'cpu+mem
;; Run the workload.
(profiler-stop)
(profiler-report)
```

The lower-level `profiler-cpu-*` and `profiler-memory-*` primitives return the
same hash-table shape consumed by GNU `profiler.el`. `profiler-log-size` and
`profiler-max-stack-depth` default to 10000 and 16.

CPU profiling uses per-thread CPU time and samples cooperatively at Lisp call
boundaries. This avoids asynchronous signal handlers entering the Rust
runtime. A long-running function is charged when it next crosses a call
boundary. Memory profiling measures bytes reported at Neomacs' managed Lisp
object-allocation points. It does not include arbitrary Rust allocations or
every later capacity change in an object's backing storage.

## Native sampling

The `profiling` Cargo profile keeps release optimizations and native debug
symbols:

```sh
cargo build --profile profiling -p neomacs
```

On Linux, record the complete process with `perf` or Samply:

```sh
perf record --call-graph dwarf ./target/profiling/neomacs -Q
perf report

samply record ./target/profiling/neomacs -Q
```

On Windows, build the same profile and record the process with Windows
Performance Recorder, then inspect the ETL trace in Windows Performance
Analyzer. This captures CPU scheduling, native stacks, allocation providers,
file I/O, and GPU activity when the corresponding WPR profiles are enabled.

```powershell
wpr -start CPU -filemode
.\target\profiling\neomacs.exe -Q
wpr -stop neomacs.etl
wpa neomacs.etl
```

For interpreter opcode frequencies, use the existing zero-default-overhead VM
instrumentation:

```sh
cargo nextest run -p neovm-core --features vm-profile --release \
  -E 'test(/vm_subr_mix_(byte_compile|fontlock)/)' --no-capture
```

The full in-tree performance panel is available at
`neovm-core/scripts/run-perf-suite.sh`. It collates every bench into one
timestamped report — JIT micro-benches (interp-vs-JIT, hot-vs-cold in one
process), GC drain-kind profiles (plain-vs-pdump A/B), allocation
round-trip probes, per-builtin VM call rankings, and real-boot AOT A/B
startup samples — so before/after numbers stop being hand-assembled. The
`vm-profile` instrumentation builds in its own feature configuration,
deliberately kept OFF for the timing sections so their numbers stay honest.

## Methodology

The tools above answer "what is hot"; the discipline below is what has made
the answers trustworthy. Every rule here was learned the hard way in a real
campaign (see the case studies).

### Deterministic workloads, batch-replayed

Interactive hot paths (font-lock, redisplay motion, reader-heavy startup)
are extracted into scripted `--batch`/`--eval` drivers that replay the same
buffer and the same operations every run. Repeatable numbers, clean stacks,
no event-loop noise drowning the profile. Profiling the live GUI session is
a last resort; replaying its workload headlessly is the default.

### Fresh build before every measurement

Profile the binary you actually built. An incremental `cargo build
--release` also invalidates the pdump fingerprint (regen with
`cargo run -p xtask -- fresh-build --release --skip-build`), and a stale
binary next to freshly edited source produces confident nonsense — the
"binary-mismatch trap". Corollary: measure BEFORE pushing; a perf claim
that was never re-measured on the final build is not a claim.

### A/B with medians, one variable at a time

Every number is old-vs-new medians over repeated runs on the same machine,
changing exactly one thing. The perf-suite benches encode this
(interleaved A/B inside one process where possible, so frequency scaling
and cache state hit both sides equally).

### Timeline questions get tracing, not a profiler

When the question is "when did X stop happening" rather than "what is
hot", use the wired-up tracing subscriber with a module-scoped filter —
e.g. `RUST_LOG=neomacs_renderer_wgpu::shader_surface_cache=trace` — and
read the timestamped timeline. No code changes, no printf, and log levels
stay untouched.

## Case studies

Real findings, each invisible in code review and obvious in a profile:

- **Gap-buffer byte↔char conversion was O(n²)** under font-lock: a
  batch-replayed font-lock driver took 4 minutes on a large buffer;
  `perf` put the time in position conversion. Fixed by porting GNU
  `marker.c`'s cached-anchor scheme: 4 min → 2 s.
- **`Value == Value` is deep equality**, so `HashSet<Value>` /
  `Vec<Value>::contains` deep-compare; a symbol-materialization loop went
  O(n²) on Doom's obarray. Flamegraph made it unmissable. Identity sets
  keyed on `bits()`: Doom startup 2.2 s → 0.52 s.
- **Doom startup phase ranking** via `perf stat` PMU breakdown + dwarf
  flamegraphs: face resolution 5.4% → 0.64% (face-list cache), reader
  8.5% → 6.4% (decode cache + contiguous-slice decode).
- **pdump load**: property-free strings made self-contained, skipping the
  object-extra table — raw `load_from_dump` median −60%, measured as
  medians over repeated loads, not single runs.

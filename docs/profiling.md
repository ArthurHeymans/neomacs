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
`neovm-core/scripts/run-perf-suite.sh`.

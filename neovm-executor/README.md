# neovm-executor

`neovm-executor` owns execution engines for compiler artifacts produced by
`neovm-compiler`.

Current scope:

- Run the runtime-free Register IR subset with the validation interpreter.
- Define the executor-side `LispValue` tagged-word ABI carrier.
- Own a small runtime heap for initial pair operations: `cons`, `car`, and
  `cdr`.
- Keep execution policy outside the compiler crate.
- Provide a development CLI:

```text
neovm-executor run [--engine=interp] <file.el> [i64-arg ...]
```

Planned scope:

- Add a Cranelift JIT engine for the runtime-free subset.
- Connect Cranelift runtime ABI imports to the executor runtime heap.
- Add runtime support for dynamic binding, symbols, closures, nonlocal control,
  and buffer-local state.
- Register stack maps and safepoint metadata for precise GC integration.

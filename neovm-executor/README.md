# neovm-executor

`neovm-executor` owns execution engines for compiler artifacts produced by
`neovm-compiler`.

Current scope:

- Run the runtime-free Register IR subset with the validation interpreter.
- Keep execution policy outside the compiler crate.
- Provide a development CLI:

```text
neovm-executor run [--engine=interp] <file.el> [i64-arg ...]
```

Planned scope:

- Add a Cranelift JIT engine for the runtime-free subset.
- Connect runtime ABI imports for heap values, dynamic binding, symbols,
  closures, nonlocal control, and buffer-local state.
- Register stack maps and safepoint metadata for precise GC integration.

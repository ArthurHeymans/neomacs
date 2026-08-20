# `Value::as_lisp_string` borrow audit

The scripts DIVERGENCES.md entry 163 used to count the `&'static LispString`
seam. They are shipped so the numbers in that entry can be re-measured rather
than believed, and so the next person auditing a borrow class has a starting
point instead of a blank file.

They are deliberately crude: line-oriented, regex-driven, and honest about it.
Every one of them OVER-approximates, and entry 163 says where each
approximation bit.

Run them from the repository root, in order:

```sh
python3 tools/gc-audit/classify2.py      > tmp/sites2.json     # every borrow site, classified
python3 tools/gc-audit/reach.py          > tmp/reach.json      # names that can reach a safepoint
python3 tools/gc-audit/ctx_and_string.py | tail -n +2 | awk '{print $2}' | sort -u \
                                         > tmp/ctxstr-names.txt
python3 tools/gc-audit/pass3.py          > tmp/pass3.json      # the filter that decides nothing
python3 tools/gc-audit/resolve_expect.py                       # borrows vs. owned clones
python3 tools/gc-audit/pass5.py                                # the final candidate list
python3 tools/gc-audit/ctx_in_scope.py                         # sizing the Tier-1 migration
```

`exp_self_lifetime.py` is not an analysis; it patches
`Value::as_lisp_string` to `(&self) -> Option<&LispString>` so `cargo check`
reports every site where a borrow escapes its `Value`, and `--revert` puts it
back.

The one number worth re-reading before trusting any of this:
`reach.py` reports that **90.2% of function names in these three crates can
reach a GC safepoint**, which is why "does this borrow cross a safepoint" is
not, on its own, a usable filter inside an interpreter.

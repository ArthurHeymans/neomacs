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
`Value::as_lisp_string` between its honest signature and the `&'static` one, so
`cargo check` reports every site where a borrow escapes its `Value`.  Entry 163
ran it to price the tightening (20 errors); 167 landed it, so the script now
LOOSENS by default and `--revert` restores the honest signature.

Every script resolves the workspace root through `gcaudit_root.py`, which
raises rather than scanning nothing.  Entry 167 found that as committed they
all pointed at `<repo>/tools`, so `classify2.py` reported `grep_lines 0 sites
0` and `reach.py` reported `0 (0.0% of defined names)` -- both exiting 0.

The one number worth re-reading before trusting any of this:
`reach.py` reports that **90.2% of function names in these three crates can
reach a GC safepoint**, which is why "does this borrow cross a safepoint" is
not, on its own, a usable filter inside an interpreter.  (Re-measured for 167:
23906 of 26518, still 90.2%.)

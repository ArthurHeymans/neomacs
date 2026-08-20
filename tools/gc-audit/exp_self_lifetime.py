#!/usr/bin/env python3
"""Ledger 163: measure the cost of the strongest ZERO-CHURN tightening of the
seam itself -- `as_lisp_string(&self) -> Option<&LispString>`.

Method-call syntax auto-refs, so every `v.as_lisp_string()` still resolves;
what stops compiling is a borrow that outlives the `Value` PLACE it came from.
This is not the safepoint property (a `Value` local can outlive a safepoint and
still be unrooted), but it is the escape property, and it is free.
"""
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
P = os.path.join(ROOT, 'neovm-core/src/emacs_core/value.rs')
OLD = "    pub fn as_lisp_string(self) -> Option<&'static LispString> {"
NEW = "    pub fn as_lisp_string(&self) -> Option<&LispString> {"

s = open(P, encoding='utf-8').read()
revert = len(sys.argv) > 1 and sys.argv[1] == '--revert'
a, b = (NEW, OLD) if revert else (OLD, NEW)
if a not in s:
    print(f"MISS: {a!r} not found")
    sys.exit(1)
open(P, 'w', encoding='utf-8').write(s.replace(a, b, 1))
print('reverted' if revert else 'applied')

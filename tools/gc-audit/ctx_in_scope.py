#!/usr/bin/env python3
"""Ledger 163: size the full Tier-1 migration.

`Context::lisp_string` can only be used where a `Context` is in scope. Count
how many of the production borrow sites are inside a function that has one.
"""
import json
import os
import re

from gcaudit_root import ROOT, require_nonzero  # noqa: E402
sites = json.load(open(os.path.join(ROOT, 'tmp/sites2.json')))['sites']
FN = re.compile(r'^(\s*)(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?'
                r'fn\s+([A-Za-z_]\w*)')
CTX_PARAM = re.compile(r'(ctx|eval|evaluator|context)\s*:\s*&\s*mut\s+(?:[A-Za-z_:]*::)?Context'
                       r'|&\s*mut\s+self\b')

cache = {}


def flines(rel):
    if rel not in cache:
        cache[rel] = open(os.path.join(ROOT, rel), encoding='utf-8').read().split('\n')
    return cache[rel]


with_ctx = 0
without = 0
without_by_file = {}
for x in sites:
    if x['cls'] in ('COMMENT', 'DEFN') or x['test']:
        continue
    lines = flines(x['file'])
    # walk up to the enclosing fn header
    start = None
    for i in range(x['line'] - 1, -1, -1):
        if FN.match(lines[i]):
            start = i
            break
    if start is None:
        without += 1
        continue
    sig, depth, started = [], 0, False
    for j in range(start, min(start + 30, len(lines))):
        sig.append(lines[j])
        for ch in lines[j]:
            if ch == '(':
                depth += 1
                started = True
            elif ch == ')':
                depth -= 1
        if started and depth <= 0:
            break
    if CTX_PARAM.search('\n'.join(sig)):
        with_ctx += 1
    else:
        without += 1
        without_by_file[x['file']] = without_by_file.get(x['file'], 0) + 1

require_nonzero('production sites', with_ctx + without)
print(f"production sites inside a fn holding a Context (or &mut self): {with_ctx}")
print(f"production sites with NO Context in scope                    : {without}")
print("top files with no Context in scope:")
for f, n in sorted(without_by_file.items(), key=lambda kv: -kv[1])[:12]:
    print(f"  {n:4d}  {f}")

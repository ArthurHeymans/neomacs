#!/usr/bin/env python3
"""Ledger 163 pass 3: for every BOUND borrow site, intersect the borrow's live
range with the safepoint-reachable call set, and classify the RECEIVER's
provenance (is the Value rooted while the borrow lives?)."""

import json
import os
import re
import sys
import collections

from gcaudit_root import ROOT  # noqa: E402  (validated workspace root)
reach = set(json.load(open(os.path.join(ROOT, 'tmp/reach.json'))))
sites = json.load(open(os.path.join(ROOT, 'tmp/sites2.json')))['sites']

CALL_RE = re.compile(r'([A-Za-z_][A-Za-z0-9_]*)\s*\(')
KEYWORDS = {'if', 'while', 'for', 'match', 'return', 'let', 'Some', 'Ok', 'Err',
            'None', 'assert', 'assert_eq', 'debug_assert', 'debug_assert_eq',
            'panic', 'vec', 'format', 'write', 'writeln', 'println', 'eprintln',
            'matches', 'as', 'move'}

# Receiver provenance: what the accessor is called on.
ROOTED_RECV = re.compile(
    r'\b(args|arg|argv|values|elts)\s*(\[|\.get\(|\.first\(|\.last\(|\.iter\()'
    r'|&args\[|args\.get\(|\bargs\b'
)
FRESH_RECV = re.compile(
    r'(Value::(string|heap_string|from_|make_)|\.clone\(\)\.as_lisp_string|'
    r'eval\.(eval|apply|funcall|call)[a-z_0-9]*\(|'
    r'\?\s*\.as_lisp_string)'
)

cache = {}


def flines(rel):
    if rel not in cache:
        with open(os.path.join(ROOT, rel), encoding='utf-8') as fh:
            cache[rel] = fh.read().split('\n')
    return cache[rel]


out = []
for x in sites:
    if x['cls'] != 'BOUND' or x['test']:
        continue
    lines = flines(x['file'])
    body = '\n'.join(lines[x['line'] - 1:x['last']])
    # calls in the live range, EXCLUDING the site line's own accessor call
    reaching = set()
    for j in range(x['line'] - 1, x['last']):
        for m in CALL_RE.finditer(lines[j]):
            n = m.group(1)
            if n in KEYWORDS or n in ('as_lisp_string', 'expect_lisp_string'):
                continue
            if n in reach:
                reaching.add(n)
    out.append({**x, 'reaching': sorted(reaching), 'nreach': len(reaching),
                'body_head': body[:0]})

print(f"production BOUND sites: {len(out)}", file=sys.stderr)
print(f"  span == 1 line (borrow dies on its own line): "
      f"{sum(1 for x in out if x['span'] == 1)}", file=sys.stderr)
print(f"  live range contains >=1 safepoint-reachable call: "
      f"{sum(1 for x in out if x['nreach'])}", file=sys.stderr)
print(f"  live range contains NO reachable call: "
      f"{sum(1 for x in out if not x['nreach'])}", file=sys.stderr)
hist = collections.Counter(min(x['span'], 20) for x in out)
print("  span histogram (lines, capped at 20):",
      dict(sorted(hist.items())), file=sys.stderr)
json.dump(out, sys.stdout, indent=1)

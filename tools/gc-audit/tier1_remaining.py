#!/usr/bin/env python3
"""Ledger 185: size what is LEFT of the Tier-1 `Context::lisp_string`
migration, and split it by the reason each site is still there.

Entry 175 sized this with a script that lived in `tmp/` and died with its
worktree, so the 63 -> 51 -> 24 arithmetic could not be re-run.  This is that
script, committed.  It reproduces 175 s5's table

    | class                          | count |
    | sites with a Context in reach  |       |
    | INLINE / OWNED / BOUND         |       |
    | of those BOUND, live range > 1 |       |

and then subtracts the two things a line-oriented classifier cannot see, both
of which 175 found by hand:

  * a site whose enclosing `&mut self` is NOT a `Context` (any type has
    `&mut self`), where `Context::lisp_string` is simply unreachable; and
  * a site that goes through a LOCAL `expect_lisp_string` helper returning an
    owned clone, which has no borrow to re-anchor -- OWNED wearing a BOUND
    label.

Run after `classify2.py`:

    python3 tools/gc-audit/classify2.py > tmp/sites2.json
    python3 tools/gc-audit/tier1_remaining.py

Like every script in this directory it OVER-approximates and says so: the
`--list` output is the input to a reading, not a verdict.
"""
import json
import os
import re
import sys

from gcaudit_root import ROOT  # noqa: E402

FN = re.compile(r'^(\s*)(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?'
                r'fn\s+([A-Za-z_]\w*)')
CTX_PARAM = re.compile(r'(ctx|eval|evaluator|context)\s*:\s*&\s*mut\s+(?:[A-Za-z_:]*::)?Context'
                       r'|&\s*mut\s+self\b')
CTX_REAL = re.compile(r'(ctx|eval|evaluator|context)\s*:\s*&\s*mut\s+(?:[A-Za-z_:]*::)?Context')
IMPL = re.compile(r'^\s*impl(?:<[^>]*>)?\s+(?:[A-Za-z_:]+\s+for\s+)?([A-Za-z_][\w:]*)')
# A site already migrated is not a remainder: it names the Context accessor.
MIGRATED = re.compile(r'\b(?:ctx|eval|evaluator|context|self)\s*\.\s*(?:expect_)?lisp_string\s*\(')
# The field-precise sibling landed by 185 counts as migrated too.
MIGRATED_HEAP = re.compile(r'\b(?:expect_)?lisp_string_in\s*\(')
# `eval.buffers.foo(...)` -- a method call on a Context FIELD rather than on
# the Context itself.  A safepoint needs `&mut Context`, so a field call can
# never reach one; it is exactly what the whole-Context anchor over-rejects.
FIELD_CALL = re.compile(r'\b(?:ctx|eval|evaluator|context)\.[a-z_]+\.[a-z_]+\s*\(')

cache = {}


def flines(rel):
    if rel not in cache:
        cache[rel] = open(os.path.join(ROOT, rel), encoding='utf-8').read().split('\n')
    return cache[rel]


def signature(lines, start):
    """The enclosing fn's WHOLE signature -- `ctx_in_scope.py`'s reading, kept
    identical so the two scripts cannot disagree about the denominator."""
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
    return '\n'.join(sig)


def enclosing(rel, line):
    """(fn header line index, impl type or None) for the site at `line`."""
    lines = flines(rel)
    start = None
    for i in range(line - 1, -1, -1):
        if FN.match(lines[i]):
            start = i
            break
    impl_ty = None
    if start is not None:
        for i in range(start, -1, -1):
            m = IMPL.match(lines[i])
            if m:
                impl_ty = m.group(1)
                break
    return start, impl_ty


def local_cloning_helper(rel):
    """True when this file owns an `expect_lisp_string` that returns an owned
    clone -- a site going through it has no borrow to anchor."""
    lines = flines(rel)
    for i, ln in enumerate(lines):
        if re.match(r'^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+expect_lisp_string\b', ln):
            body = '\n'.join(lines[i:i + 14])
            if '.clone()' in body or 'to_owned()' in body or '-> Result<LispString' in body:
                return True
    return False


def main():
    sites = json.load(open(os.path.join(ROOT, 'tmp/sites2.json')))['sites']
    tally = {'with_ctx': 0, 'INLINE': 0, 'OWNED': 0, 'BOUND': 0, 'multiline': 0}
    remaining = []
    excluded_no_ctx = []
    excluded_cloning = []
    for x in sites:
        if x['cls'] in ('COMMENT', 'DEFN') or x['test']:
            continue
        lines = flines(x['file'])
        start, impl_ty = enclosing(x['file'], x['line'])
        if start is None:
            continue
        sig = signature(lines, start)
        if not CTX_PARAM.search(sig):
            continue
        tally['with_ctx'] += 1
        tally[x['cls']] = tally.get(x['cls'], 0) + 1
        if x['cls'] != 'BOUND':
            continue
        if x['last'] > x['line']:
            tally['multiline'] += 1
        text = lines[x['line'] - 1]
        if MIGRATED.search(text) or MIGRATED_HEAP.search(text):
            continue
        row = f"{x['file']}:{x['line']}"
        # `&mut self` is any type's method; only a real Context reaches the
        # accessor.
        impl_is_context = impl_ty is not None and impl_ty.split('::')[-1] == 'Context'
        if not CTX_REAL.search(sig) and impl_ty is not None and not impl_is_context:
            excluded_no_ctx.append(f'{row}  (impl {impl_ty})')
            continue
        if local_cloning_helper(x['file']) and 'expect_lisp_string' in text:
            excluded_cloning.append(row)
            continue
        # Does the borrow's live range touch a DISJOINT `Context` field?  That
        # is the shape `Context::lisp_string` rejects wrongly (175 s5's one
        # measured false positive) and `Value::expect_lisp_string_in` accepts,
        # so it sizes the class the field-precise accessor exists for.
        live = '\n'.join(lines[x['line'] - 1:x['last']])
        disjoint = bool(FIELD_CALL.search(live))
        remaining.append((row, disjoint))

    print('175 s5 table, re-measured')
    print(f"  sites with a Context in reach : {tally['with_ctx']}")
    print(f"  INLINE                        : {tally['INLINE']}")
    print(f"  OWNED                         : {tally['OWNED']}")
    print(f"  BOUND                         : {tally['BOUND']}")
    print(f"  of those, live range > 1 line : {tally['multiline']}")
    print()
    candidates = len(remaining) + len(excluded_no_ctx) + len(excluded_cloning)
    print(f'BOUND, not yet on a Context/heap accessor : {candidates}')
    print(f'  minus: enclosing &mut self is not a Context : {len(excluded_no_ctx)}')
    print(f'  minus: goes through a local CLONING helper  : {len(excluded_cloning)}')
    print(f'ELIGIBLE REMAINDER                        : {len(remaining)}')
    n_disjoint = sum(1 for _, d in remaining if d)
    print(f'  of those, live range touches a disjoint Context field : {n_disjoint}')
    print('  (that is the class `Value::expect_lisp_string_in` serves; the')
    print('   whole-Context anchor rejects them and the compiler is right to)')
    if '--list' in sys.argv:
        print()
        for row, disjoint in remaining:
            print('  ', row, '   <- touches a disjoint Context field' if disjoint else '')
        print('  -- excluded (no Context) --')
        for row in excluded_no_ctx:
            print('  ', row)
        print('  -- excluded (cloning helper) --')
        for row in excluded_cloning:
            print('  ', row)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())

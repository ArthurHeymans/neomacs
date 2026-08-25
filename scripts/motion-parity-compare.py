#!/usr/bin/env python3
"""Diff two scripts/motion-parity-audit.el outputs (ledger 195).

  scripts/motion-parity-compare.py GNU.txt NEOMACS.txt [-v]

Reports the divergence count broken down by config and by motion.  The two
`posn-*` motions are CONTROLS: they ask the layout engine rather than
`vertical-motion', so a divergence there means the rows themselves differ and
not the motion over them (ledger 184's rule).
"""
import sys, collections
gnu, neo = sys.argv[1], sys.argv[2]
def load(p):
    d = {}
    for line in open(p):
        line = line.rstrip("\n")
        if line.startswith("CONFIG") or not line:
            continue
        cfg, pos, motion, val = line.split("|", 3)
        d[(cfg, pos, motion)] = val
    return d
g, n = load(gnu), load(neo)
keys = sorted(set(g) | set(n))
div = [k for k in keys if g.get(k) != n.get(k)]
print(f"probes total={len(keys)} divergent={len(div)} agreeing={len(keys)-len(div)}")
by_cfg = collections.Counter(k[0] for k in div)
by_mot = collections.Counter(k[2] for k in div)
print("\nby config:")
for cfg, _ in sorted(collections.Counter(k[0] for k in keys).items()):
    print(f"  {cfg:28s} {by_cfg.get(cfg,0):4d} / {sum(1 for k in keys if k[0]==cfg)}")
print("\nby motion:")
for mot in sorted(set(k[2] for k in keys)):
    print(f"  {mot:14s} {by_mot.get(mot,0):4d} / {sum(1 for k in keys if k[2]==mot)}")
if len(sys.argv) > 3 and sys.argv[3] == "-v":
    print("\nfirst 60 divergences (config|pos|motion  GNU -> NEO):")
    for k in div[:60]:
        print(f"  {k[0]}|{k[1]}|{k[2]}  {g.get(k)!r} -> {n.get(k)!r}")

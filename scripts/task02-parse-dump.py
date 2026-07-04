#!/usr/bin/env python3
"""Parse SUBR-MIX dumps (vm_profile::report format) from task-02 logs.

Extracts, per SUBR-MIX section: builtin -> (calls, pct, opcall, cbsym, cbtin, other).
Prints each section's top rows and, if multiple sections/files given, a merged
union ranking. Usage: parse_dump.py LOG [LOG ...]
"""
import re
import sys

SEC = re.compile(r"=== SUBR-MIX \[(?P<label>.*?)\]: (?P<total>\d+) builtin calls, (?P<distinct>\d+) distinct ===")
# row: name calls pct% opcall cbsym cbtin other
ROW = re.compile(
    r"^\s{2}(?P<name>\S+)\s+(?P<calls>\d+)\s+(?P<pct>[\d.]+)%\s+"
    r"(?P<opcall>\d+)\s+(?P<cbsym>\d+)\s+(?P<cbtin>\d+)\s+(?P<other>\d+)\s*$"
)

def parse(path):
    sections = []
    cur = None
    with open(path) as f:
        for line in f:
            m = SEC.search(line)
            if m:
                cur = {"label": m.group("label"), "total": int(m.group("total")),
                        "rows": {}}
                sections.append(cur)
                continue
            if cur is not None:
                r = ROW.match(line.rstrip("\n"))
                if r:
                    cur["rows"][r.group("name")] = dict(
                        calls=int(r.group("calls")), pct=float(r.group("pct")),
                        opcall=int(r.group("opcall")), cbsym=int(r.group("cbsym")),
                        cbtin=int(r.group("cbtin")), other=int(r.group("other")))
    return sections

def entry_kind(row):
    c = row["calls"] or 1
    parts = []
    if row["opcall"]:  parts.append(("Op::Call", row["opcall"]))
    if row["cbsym"]:   parts.append(("CBSym",    row["cbsym"]))
    if row["cbtin"]:   parts.append(("CBtin",    row["cbtin"]))
    if row["other"]:   parts.append(("other",    row["other"]))
    parts.sort(key=lambda x: -x[1])
    return ", ".join(f"{k} {100*v/c:.0f}%" for k, v in parts) or "-"

def show(sec, n=40):
    print(f"\n### SUBR-MIX [{sec['label']}]  total={sec['total']:,}  distinct={len(sec['rows'])}")
    print(f"  {'rank':>4} {'builtin':<26} {'calls':>12} {'%':>6}  entry-split")
    rows = sorted(sec["rows"].items(), key=lambda kv: -kv[1]["calls"])
    for i, (name, r) in enumerate(rows[:n], 1):
        print(f"  {i:>4} {name:<26} {r['calls']:>12,} {r['pct']:>5.2f}%  {entry_kind(r)}")

def main():
    files = sys.argv[1:]
    if not files:
        print(__doc__); return
    all_secs = []
    for p in files:
        secs = parse(p)
        for s in secs:
            s["file"] = p
            all_secs.append(s)
            show(s)
    print(f"\nparsed {len(all_secs)} SUBR-MIX section(s) from {len(files)} file(s)")

if __name__ == "__main__":
    main()

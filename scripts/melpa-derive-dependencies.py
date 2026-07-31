#!/usr/bin/env python3
"""Derive dependency cells in `melpa-package-lock.tsv` from each pinned
package's own `Package-Requires` header, and report lockfile disagreements.

Each package lock row records its direct dependencies as a sorted comma-separated
list of package names, or `-` when it has none. Versions are deliberately absent:
the lock pins every package at exactly one version, so carrying versions on each
edge would be derivable data that can disagree with the owning source row.

An edge is included when the dependency is *pinned in the source lock*.  A
requirement that is not pinned is either built into Emacs (`cl-lib`, `json`,
`flymake`, `seq`) or is something the suite never installs, and in both cases
package.el resolves it without our help.

Usage:
    scripts/melpa-derive-dependencies.py            # report differences
    scripts/melpa-derive-dependencies.py --write    # rewrite the manifest

Coverage is limited to packages whose main `.el` is present in the local caches
below `tmp/melpa`, so a full regeneration needs the packages built first.  With
partial coverage `--write` refuses to drop edges it could not confirm, since an
absent cache entry is not evidence that an edge is wrong.
"""

from __future__ import annotations

import argparse
import collections
import os
import pathlib
import re
import sys

WORKSPACE = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = WORKSPACE / "neomacs-melpa-tests/melpa-package-lock.tsv"
CACHES = ("tmp/melpa/source-install-cache", "tmp/melpa/package-cache")

HEADER = (
    "package\tversion\tupstream\tupstream-revision\trepository\trevision\t"
    "fallback-repository\tbuild\tdependencies"
)
REQUIRES = re.compile(r";;\s*Package-Requires:\s*(.+)$", re.M)
REQUIREMENT = re.compile(r"\(\s*([A-Za-z0-9@_.+-]+)\s")


def read_manifest() -> tuple[list[list[str]], dict[str, str], dict[str, set[str]]]:
    rows: list[list[str]] = []
    versions: dict[str, str] = {}
    edges: dict[str, set[str]] = collections.defaultdict(set)
    with MANIFEST.open(encoding="utf-8") as handle:
        if next(handle).rstrip("\n") != HEADER:
            sys.exit(f"{MANIFEST} does not start with the expected package-lock header")
        for line in handle:
            fields = line.rstrip("\n").split("\t")
            if len(fields) != 9:
                sys.exit(f"{MANIFEST} contains a row with {len(fields)} fields")
            if fields[0] in versions:
                sys.exit(f"{fields[0]} is pinned at more than one version")
            rows.append(fields)
            versions[fields[0]] = fields[1]
            if fields[8] != "-":
                dependencies = fields[8].split(",")
                if dependencies != sorted(set(dependencies)):
                    sys.exit(f"{fields[0]} dependencies are not sorted and unique")
                edges[fields[0]].update(dependencies)
    return rows, versions, edges


def cached_main_files(versions: dict[str, str]) -> dict[str, pathlib.Path]:
    wanted = {f"{name}-{version}": name for name, version in versions.items()}
    found: dict[str, pathlib.Path] = {}
    for cache in CACHES:
        for directory, _, files in os.walk(WORKSPACE / cache):
            name = wanted.get(os.path.basename(directory))
            if name and f"{name}.el" in files and name not in found:
                found[name] = pathlib.Path(directory) / f"{name}.el"
    return found


def derived_edges(
    versions: dict[str, str], sources: dict[str, pathlib.Path]
) -> tuple[dict[str, set[str]], dict[str, set[str]]]:
    """Return (edges to pinned packages, requirements that are not pinned)."""
    edges: dict[str, set[str]] = collections.defaultdict(set)
    unpinned: dict[str, set[str]] = collections.defaultdict(set)
    for name, path in sorted(sources.items()):
        header = REQUIRES.search(path.read_text(encoding="utf-8", errors="replace")[:8000])
        if not header:
            continue
        for requirement in REQUIREMENT.findall(header.group(1)):
            if requirement == "emacs" or requirement == name:
                continue
            if requirement in versions:
                edges[name].add(requirement)
            else:
                unpinned[name].add(requirement)
    return edges, unpinned


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="rewrite the manifest")
    arguments = parser.parse_args()

    rows, versions, committed = read_manifest()
    sources = cached_main_files(versions)
    derived, unpinned = derived_edges(versions, sources)

    missing = {
        package: sorted(dependencies - committed.get(package, set()))
        for package, dependencies in derived.items()
        if dependencies - committed.get(package, set())
    }
    spurious = {
        package: sorted(committed[package] - derived[package])
        for package in derived
        if committed.get(package, set()) - derived[package]
    }

    print(f"pinned packages           : {len(versions)}")
    print(f"main .el readable in cache: {len(sources)}")
    print(f"declaring Package-Requires: {len(derived) + len(unpinned)}")
    print(f"packages with pinned edges: {len(derived)}")

    if missing:
        print(f"\nderived but not committed ({sum(map(len, missing.values()))}):")
        for package, dependencies in sorted(missing.items()):
            print(f"  {package} -> {', '.join(dependencies)}")
    if spurious:
        print(f"\ncommitted but not derived ({sum(map(len, spurious.values()))}):")
        for package, dependencies in sorted(spurious.items()):
            print(f"  {package} -> {', '.join(dependencies)}")
    if not missing and not spurious:
        print("\nthe committed manifest matches every edge that could be derived")

    if arguments.write:
        # Only add what was derived; never drop an edge whose package could not
        # be read, because an absent cache entry is not evidence.
        merged = {package: set(dependencies) for package, dependencies in committed.items()}
        for package, dependencies in derived.items():
            merged.setdefault(package, set()).update(dependencies)
            if package in sources:
                merged[package] = set(dependencies)
        for row in rows:
            row[8] = ",".join(sorted(merged.get(row[0], set()))) or "-"
        contents = "\n".join([HEADER, *("\t".join(row) for row in rows)]) + "\n"
        MANIFEST.write_text(contents, encoding="utf-8")
        edge_count = sum(len(dependencies) for dependencies in merged.values())
        print(f"\nwrote {edge_count} edges to {MANIFEST.relative_to(WORKSPACE)}")

    return 1 if (missing or spurious) and not arguments.write else 0


if __name__ == "__main__":
    raise SystemExit(main())

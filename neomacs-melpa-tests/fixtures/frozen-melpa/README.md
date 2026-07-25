# Frozen MELPA compatibility archive

This directory is an immutable, offline package source for compatibility
tests. The tarballs were downloaded unmodified from `https://melpa.org/packages/`
on 2026-07-25 and are pinned by `SHA256SUMS`.

Refresh this snapshot with `cargo xtask refresh-melpa-fixtures`.
The command rejects packages already built into Neomacs and validates
package name, version, commit, license, and checksum metadata.

| Package | MELPA version | Upstream commit | License |
|---|---:|---|---|
| dash | 20260221.1346 | `d3a84021dbe48dba63b52ef7665651e0cf02e915` | GPL-3.0-or-later |
| hydra | 20250316.1254 | `59a2a45a35027948476d1d7751b0f0215b1e61aa` | GPL-3.0-or-later |
| lv | 20200507.1518 | `87873d788891029d9e44fa5458321d6a05849b94` | GPL-3.0-or-later |
| s | 20220902.1511 | `b4b8c03fcef316a27f75633fe4bb990aeff6e705` | GPL-3.0-or-later |

The packages retain their upstream copyright and licensing headers. They
are test fixtures, not runtime dependencies.

# Neomacs test fonts

This dev-only crate downloads the pinned Spleen 2.2.0 release and a
commit-pinned W3C WOFF2 collection test on first use, verifies their SHA-256
digests, and caches the selected font files under `./tmp/font-fixtures`. No
font binaries are stored in the Git repository.

Tests deliberately fail if the download or integrity check fails. The cache is
shared across workspace test processes and guarded by a file lock. Delete
`./tmp/font-fixtures` to force a clean download.

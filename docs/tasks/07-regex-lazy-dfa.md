# Task 07 — Regex lazy-DFA fast path

Status: EVIDENCE-BASED DEFER after a full two-critic adjudication. This doc
records WHY (so nobody re-proposes the naive version), the PREREQUISITE BUG
any future attempt must fix first, and the exact re-scoped increment that
WOULD be sound if the trigger ever fires. Risk if built as originally
sketched: silent wrong matches + a net perf REGRESSION on the headline
pattern. Effort if revived: the honest version is an irregexp-scale project.

## 1. Context: what already happened to the regex engine

The engine is a faithful GNU regex-emacs.c port: bytecode compiler +
interpreted backtracker (`emacs_core/regex_emacs.rs`), pattern LRU caches in
`regex.rs`. The 2026-07 work landed two big commits:
- fastmap restored for syntax/class patterns (syntax-table-keyed cache +
  mutation epoch),
- GNU failure-stack protocol (reusable scratch, delta register saves),
  OnFailureJumpSmart, memchr-accelerated skip (SIMD via memchr/2/3 on <=3-byte
  first-byte sets — ALL 8 real font-lock patterns qualify), gap-motion search,
  and the `re_max_failures` stack limit (pathological patterns now signal
  like GNU instead of unbounded churn).

Measured: one full fontify pass over 256KiB of real elisp with the real GNU-31
lisp-mode font-lock regexps: **146.8ms -> 14.5ms (10.1x)**. GNU C reference
~4.5ms; rust-regex-crate structural bound ~0.69ms. The lazy-DFA idea targeted
the remaining 14.5 -> ~2.4ms.

## 2. Why it was deferred (the two-critic convergence)

### 2a. The semantics critic (design-killer)
6 of the 8 real font-lock patterns use `\w`, `\s_`, `\_<`, `\_>` (the
symbol-scanning core `\(?:\w\|\s_\|\\.\)+`), and even `[[:alnum:]]` is
Unicode-aware. The standard syntax table maps THE ENTIRE non-ASCII range
(0x80..=0x3FFFFF) to Word. Two readings of the "lower syntax classes to byte
classes" gate:
- LITERAL (reject non-ASCII-decidable): rejects 6/8 patterns -> the fast path
  misses its own workload. Self-defeating.
- CHARITABLE: build a Unicode-class-over-live-mutable-char-table -> UTF-8 byte
  sub-automaton compiler (what irregexp actually is), PLUS an unconditional
  fallback whenever the region carries syntax-table TEXT PROPERTIES
  (position-dependent syntax a DFA cannot represent), PLUS epoch re-keying,
  PLUS `\s_` is NOT uniform over non-ASCII so the full table scan is needed
  anyway. Feasible in principle; an order of magnitude more work than the
  sketch. `.`/`[^...]` must also be UTF-8-char-structured (a raw-byte `.`
  splits multibyte chars). Case-fold: the translate table is a full
  codepoint map, NOT length-preserving (U+212A KELVIN -> k) -> byte-level
  folding is sound ONLY for ASCII; `case_fold && multibyte` must gate out
  (hits isearch, not lisp font-lock).
Also PINNED as sound: leftmost-first via a PRIORITY-ORDERED thread-set DFA
(RE2 Perl mode) — never a classic unordered subset DFA (`a\|ab` on "ab" must
yield "a"); confined-captures equivalence holds GIVEN the true leftmost-first
end + an API of (original_text, absolute_start, region_edges) — never a
slice (word-boundary/anchor context) — with the confined run doubling as a
free self-check on the DFA's end. POSIX-mode functions (leftmost-longest,
~0.4% of call sites) gate to the backtracker cleanly.

### 2b. The mechanics critic (perf refutation + a latent bug)
- **Perf is bimodal.** I (the critic) counted the actual haystack: sexp-head-kw
  has ~6412 candidates : 6004 matches (94% hit rate) and its capture WRAPS
  the quantifier. The DFA-span + confined-backtracker-captures architecture
  RE-EXECUTES the symbol-scanning loop inside every match — the DFA does a
  full 256KiB pass AND the per-match work stays. NET REGRESSION on the
  headline pattern. Only a TAGGED DFA (captures during the scan) fixes that
  class. The winning bucket is low-density literal-trie patterns (el-defs
  13:1, autoload-cookie, el-errors, catch-throw): ~2-4x on those only.
- **memchr already owns the skip.** All 8 patterns have <=3-byte first sets;
  the landed sparse-ASCII fastmap uses SIMD memchr. A byte-at-a-time DFA
  self-loop LOSES to it on miss regions; any DFA integration must KEEP the
  memchr prefix acceleration and feed the DFA from candidates (RE2 does
  exactly this).
- **CONFIRMED latent bug (the prerequisite):** bare `\w`/`\s_` patterns set
  `uses_syntax` (informational) but NOT `used_syntax` (the cache-keying
  flag) because their BYTECODE is table-independent (syntax resolved
  per-call). So their cache entries carry `entry_syntax_key = None`, and
  `syntax_key_matches(None, _) == true` UNCONDITIONALLY. Harmless today —
  the backtracker re-reads the live table every call. FATAL for any future
  DFA that BAKES the table: a DFA built against buffer A's syntax table
  would be served for buffer B and would survive `modify-syntax-entry`.
  **Any table-baking artifact must force a real `Some(syntax_key)` for
  DFA-lowered patterns + a modified-syntax-table regression test.**
- Gap-segment state carry is MOOT: this port is single-segment (`re_match_2`
  unimplemented; searches get contiguous bytes via gap motion).
- ReDoS-safety — the DFA's classic headline value — is already covered by the
  landed `re_max_failures` signal.

## 3. The trigger to revive

Revive ONLY if: (a) profiled jit-lock CHUNK latency (not full-pass numbers)
on real sessions shows the regex engine as the measured frontier AFTER the
display-side work (task 04), AND (b) someone is prepared to fund either the
UTF-8 class compiler (charitable-gate) or a TDFA (for the capture-wrapping
patterns) as a first-class project. Note GNU C itself is only ~3x ahead of
the current engine; consider closing THAT gap with more backtracker
micro-work first (profile where the remaining 14.5ms goes — the last
attribution showed dispatch ~27-35% and residual state churn).

## 4. The re-scoped increment that WOULD be approved (if revived)

1. FIX THE CACHE KEY FIRST (§2b prerequisite) + its regression test.
2. Gate + bytecode-program-walk NFA builder (reuse the `compile_fastmap`
   worklist-traversal shape — it already interprets every control op:
   Jump as edge, OnFailureJump* family as prioritized splits, SucceedN/JumpN
   as counters). Gate-rejects: backref/`Duplicate`, `\=`/AtDot,
   `\c`/`\C` category ops, `case_fold && multibyte`, POSIX calls, backward
   searches, intervals beyond an unroll cap, `\w`/`\s_` over multibyte
   targets unless the table is uniform over non-ASCII.
3. PRIORITY-ordered thread-set DFA, forward-only; keep memchr prefix
   acceleration in front; captures via the confined backtracker with the
   (original_text, absolute positions, region_edges) API + the self-check.
4. Target ONLY the low-density trie bucket; explicitly measure sexp-head-kw
   for regression and keep the DFA OFF for single-greedy-capture-over-
   quantifier patterns until a TDFA exists.
5. Equivalence fuzz as the merge gate: the `assert_fastmap_equivalence` +
   `FORCE_DISABLE_FASTMAP` harness is the exact template (position + register
   + existence across span variants); add non-greedy priority cases
   (autoload-cookie's `+?`), a modified-syntax-table case, and gate-reject
   coverage.
6. Honest expectations table: fontlock_engine blended +2-4x on trie patterns;
   sexp-head-kw flat-to-worse (gated off); string-match small-string bench
   FLAT (per-call-overhead bound); backtracking bench big win only on
   pathological inputs (already signal-bounded).

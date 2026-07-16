# neovm-core layout: the four-population design

Status: adopted 2026-07-16 (this document is the placement authority; the
migration is incremental and tracked in the tables below).

## The problem

neovm-core ports GNU Emacs's C core (152 `src/*.c` files, ~1,700 `DEFUN`s) to
Rust while also carrying machinery GNU never had (tagged-pointer GC, Cranelift
JIT, pdump, layout-engine bridges). As of 2026-07-16 the crate registers
1,575 unique elisp-visible subrs across 355 files / 577k lines, and four
different kinds of code share one directory namespace:

- ports of GNU C `DEFUN`s (1,492 subrs — the compatibility surface),
- Rust reimplementations of functions GNU defines in **Lisp** (52 subrs
  shadowing `subr.el`, `window.el`, `simple.el`, `kmacro.el`, …),
- neomacs-only surface (31 subrs: `neomacs-*`, `neovm-*`, plus a few
  un-namespaced accidents),
- internal machinery with no elisp surface at all.

The `DEFUN` metadata GNU keeps in one macro invocation (name, arity,
interactive spec, docstring, body) is spread over six name-keyed places here:
the body fn, the `ctx.defsubr(...)` registration site, the
`SUBR_ARITY_ORACLE` override table (`subr_info.rs`), the docstring table
(`subr_docs/gnu_table.rs`), the intspec table (`interactive.rs`), and the
special-form classification lists (`subr_info.rs`).

## The organizing principle

> **GNU's `src/` layout is the schema.** One GNU C file ↔ one Rust module of
> the same name (a file, or a directory when the port needs subdividing).
> Everything that is not a port of a GNU C file lives outside the mirror
> tree, in a home that states what it is.

This is not aesthetics; it is what makes parity auditable. Given any GNU
DEFUN, its Rust port must be findable without grep archaeology, and given any
Rust module, its upstream reference must be unambiguous. GNU itself builds
this way: `make-docfile` scans `DEFUN`s to generate `etc/DOC`, `globals.h` is
generated from `DEFVAR` scans. Our generated tables (`subr_docs/gnu_table.rs`,
`var_docs/gnu_table.rs`) are the same architecture.

## The four populations and their homes

| Population | Definition | Home |
|---|---|---|
| **A — mirror** | port of one GNU `src/<name>.c` | `emacs_core/<name>.rs` or `emacs_core/<name>/` |
| **B — machinery** | internal; no elisp surface beyond `neovm--*` debug hooks | outside `emacs_core/` (e.g. `tagged/`), or `emacs_core/internal/` until moved |
| **C — neomacs surface** | intentional neomacs-only subrs, `neomacs-*`/`neovm-*` namespaced | `emacs_core/neo/` |
| **D — lisp shims** | Rust reimplementations of functions GNU defines in `lisp/**/*.el` | `emacs_core/shims/` — **quarantined, goal is deletion** |

Population D exists only because bootstrap once needed it. The project rule
(GNU parity) says Lisp functionality comes from loading the real `.el` file,
not from reimplementation. Every shim file must carry a header naming the
`.el` it shadows and what blocks deleting it. Note that `defsubr_with_entry`
already refuses to clobber a non-subr function cell on the pdump path — so
many shims are dead weight after loadup; the runtime-probe table below shows
which are still live divergences.

### Placement rules (for humans and dispatched agents)

1. Porting a `DEFUN` from GNU `src/foo.c`? It goes in `emacs_core/foo.rs`
   (create the file/dir if missing) — never in `builtins/`, `misc.rs`, or the
   file you happen to have open.
2. Pure-Rust helper used by one mirror module? Private fn in that module.
   Used by many? It is machinery (population B) — put it outside the mirror.
3. New neomacs-visible function? Must be `neomacs-*`/`neovm--*` namespaced and
   live in `emacs_core/neo/`. Un-namespaced inventions break user code that
   feature-detects with `fboundp`.
4. Never reimplement a function GNU defines in Lisp. Load the `.el`. If
   bootstrap genuinely requires a Rust stand-in, it goes in
   `emacs_core/shims/` with a tracking header, and the `.el` definition must
   win once loaded.
5. Registration lives next to the code: each mirror module owns
   `pub(crate) fn syms_of_<name>(ctx)` (GNU's `syms_of_foo` pattern)
   registering its subrs, defvars, and defsyms. The central registrar only
   sequences `syms_of_*` calls in GNU `emacs.c` order.

## Current inventory (measured 2026-07-16)

Extraction: one-off analysis scripts under git-ignored `tmp/`
(`extract_subr_map.py`, `analyze_subr_map.py`); numbers below are from the
2026-07-16 run plus a 13-agent verification pass.

- 1,578 registrations / 1,575 unique names (3 double registrations:
  `bare-symbol`, `markerp`, `nconc` — all in `builtins/mod.rs`).
- 1,492 C ports; 245 GNU DEFUNs unported (triage table below).
- 52 EL-shadow subrs; 31 neomacs-only names.
- `builtins::init_builtins` is one flat ~8,300-line function making ~1,620
  registration calls for subrs whose bodies live across 20 `builtins/*.rs`
  grab-bag files (symbols.rs alone: 102 bodies / 134 subrs / 20+ GNU homes) —
  the single biggest violation of the mirror principle.
- ~130 C ports (heuristic count) live in a mirror-named module that is not
  their GNU home; agent-verified examples: `font.rs` mixes `font.c` with
  `xfaces.c`, `editfns.rs` carries `cmds.c`/`buffer.c`/`fns.c` strays,
  `advice.rs` is really `data.c`'s variable-watcher surface.

## Module classification (measured + agent-verified 2026-07-16)
122 modules classified. Full evidence: workflow `classify-neovm-modules` (session data).
### Clean mirrors (A) — keep as-is
`alloc.rs`, `buffer/`, `bytecode/`, `casetab.rs`, `category.rs`, `ccl.rs`, `comp.rs`, `composite.rs`, `dired.rs`, `dispnew/`, `doc.rs`, `fileio.rs`, `filelock.rs`, `floatfns.rs`, `fns.rs`, `fontset.rs`, `image.rs`, `indent.rs`, `json.rs`, `keyboard.rs`, `keyboard/`, `keymap.rs`, `print.rs`, `profiler.rs`, `regex_emacs.rs`, `sound.rs`, `sqlite.rs`, `syntax.rs`, `threads.rs`, `timefns.rs`, `xfaces/`, `xml.rs`, `xwidget.rs`
### Misnamed mirrors (A_MISNAMED) — rename/merge into the true GNU home
| module | actually ports | action |
|---|---|---|
| `advice.rs` | `data.c` | Rename/move into the data.c mirror module (e.g. data/watchers.rs); the 'advice' name is misleading. |
| `buffer_vars.rs` | `buffer.c` | Merge into the buffer/ mirror directory (e.g. buffer/vars.rs). |
| `custom.rs` | `data.c` | Rename/merge into the data.c mirror (data/ dir, buffer-local section); move buffer-local-variables to the buffer.c mirror and neomacs-set-bu |
| `dbus.rs` | `dbusbind.c` | Rename to dbusbind.rs (or keep name with a header note) — otherwise a clean single-file mirror. |
| `dynamic_module.rs` | `emacs-module.c` | Rename to emacs_module.rs to match GNU src/emacs-module.c; pull the module-load registration body reference alongside. |
| `emacs_char.rs` | `character.c` | Rename/merge into a character.rs mirror of character.c/character.h and gather the character.c subr bodies (currently in encoding and builtin |
| `format.rs` | `timefns.c` | Merge into the timefns mirror module (emacs_core/timefns.rs) as its format-time-string section; fix the stale module doc. |
| `hscroll.rs` | `xdisp.c` | It is a slice of xdisp.c named after the feature rather than the file; house it under the redisplay/xdisp directory subdivision (valid A lay |
| `regex.rs` | `search.c` | It is the engine half of the search.c mirror misnamed 'regex'; merge/rename into a search.c mirror module (alongside search.rs), keeping reg |
| `subr_info.rs` | `data.c` | Fold into the data.c mirror module (or keep as a named data.c sub-slice, documented as such). |
| `timer.rs` | `dispnew.c` | Move builtin_sleep_for into the dispnew mirror module (or rename the file to make the dispnew.c origin explicit); the timer-batch capture he |
| `windows.rs` | `w32proc.c` | Rename to w32proc.rs (or w32/) so the mirror naming matches GNU's src/w32proc.c and avoids confusion with window management. |
| `zlib.rs` | `decompress.c` | Rename to decompress.rs to match the GNU home file. |

### Pure lisp-shims (D) — quarantine in `shims/`, goal: delete
| module | shadows | note |
|---|---|---|
| `abbrev.rs` | lisp/abbrev.el | GNU has no src/abbrev.c; define-abbrev, abbrev-table-p, abbrev-symbol, abbrev-expansion, clear-abbrev-table are all defined in lisp/abbrev.el. Rust bu |
| `bookmark.rs` | lisp/bookmark.el | GNU has no src/bookmark.c; bookmark-set/jump/delete/rename/all-names/save/load are all lisp/bookmark.el. The Rust builtin_bookmark_* fns carry 13 '#[a |
| `cl_lib.rs` | emacs-lisp/cl-lib.el, emacs-lisp/cl-seq.el, emacs-lisp/seq.el | Header: 'CL-lib, seq.el, and JSON built-in functions... Common Lisp compatibility functions'; ~50 builtin_cl_*/builtin_seq_* fns (cl-first, cl-remove- |
| `isearch.rs` | isearch.el, replace.el | Rust reimplementation of the incremental-search and query-replace state machines (begin_search/add_char/toggle_regexp/lazy-highlight, QueryReplace res |
| `rect.rs` | rect.el | Header lists extract-rectangle, kill-rectangle, yank-rectangle, string-rectangle etc. — every one a defun in GNU lisp/rect.el (no src/rect.c exists; c |
| `register.rs` | register.el | All nine builtins (builtin_copy_to_register, insert/point/number/increment-register, view-register, get/set-register, register-to-string) are defuns i |

### Mixed modules — split required (the migration backlog)
| module | homes | split |
|---|---|---|
| `autoload.rs` | eval.c + lisp/subr.el, lisp/emacs-lisp/byte-run.el | Split: builtin_autoload/builtin_autoload_do_load → eval.c mirror module; symbol-file, with-eval-after-load, obsolete-alias shims → lisp-shim population (load subr.el/byte-run.el instead). |
| `builtins_extra.rs` | editfns.c, data.c, callint.c | Dissolve: editfns.c fns (propertize, system-name, user-*-name) → editfns mirror; bare-symbol/closurep (+1 more data.c fn) → data mirror; prefix-numeric-value → callint mirror; runtime_identity helpers follow the editfns  |
| `callproc/` | callproc.c + lisp/subr.el | Split: keep call-process/call-process-region/process-file + spawn machinery as the callproc.c mirror (A); move the *-shell-command and process-lines* wrappers to the lisp-shim population (load subr.el definitions instead |
| `casefiddle.rs` | casefiddle.c, character.c | Essentially an A mirror of casefiddle.c: keep the 9 case fns; relocate builtin char-resolve-modifiers to the character/ module. |
| `charset.rs` | charset.c, coding.c | Mostly the charset.c mirror; move the 4 sjis/big5 builtins to the coding.c mirror, rest stays. |
| `chartable.rs` | chartab.c, data.c, alloc.c | Split: char-table half stays as the chartab.c mirror (rename to chartab.rs for name parity); bool-vector fns move to the data.c/alloc.c mirror homes (data/ dir). |
| `coding.rs` | coding.c, doc.c, textconv.c + international/mule.el | Keep as coding.c mirror; move text-quoting-style to the doc.c mirror, set-text-conversion-style to a textconv module, and replace the set-buffer-file-coding-system shim by loading mule.el. |
| `display.rs` | xfns.c, xselect.c, menu.c, frame.c, xfaces.c + frame.el, term/pc-win.el | Split four ways: xfns-family display/frame-geometry/tooltip fns stay here as the unified window-system-fns mirror (neomacs's single-GUI analog of xfns.c); selection fns to an xselect.c mirror; x-popup-menu/dialog to a me |
| `editfns.rs` | editfns.c, cmds.c, buffer.c, fns.c, data.c | Keep the editfns.c core as the editfns mirror; move builtin_delete_char to a cmds module, builtin_erase_buffer to buffer, builtin_load_average to fns, builtin_logcount to data. |
| `errors.rs` | eval.c, print.c, data.c + lisp/subr.el (define-error) | Move builtin_signal into the eval.c mirror, builtin_error_message_string into a print mirror; keep init_standard_errors as the data.c syms_of_data port; define_error helpers should defer to loading subr.el where possible |
| `eval.rs` | eval.c, lread.c | Split: keep specpdl/specbind/eval/apply/condition-case as the eval.c mirror; extract the Context state container + GC control + display/input wiring into internal VM machinery (population B); move load_file_internal* int |
| `font.rs` | font.c, xfaces.c + lisp/faces.el (color-defined-p, color-values) | Split into two mirrors: keep font.rs for the font.c subrs; move the entire face/color half (internal-*-lisp-face + color-*) into a new xfaces.rs; xw-color-* can live with xfaces as the platform color backend; consider lo |
| `frame_vars.rs` | frame.c, xdisp.c + startup.el | Dissolve: frame.c vars -> the frame mirror module's syms_of section; xdisp.c vars -> the redisplay/xdisp vars section; handle-args-function* -> the startup.el shim/bootstrap layer. Keep the deliberate NEO-Emacs title-bra |
| `hashtab.rs` | fns.c, lread.c + subr-x.el | Core stays as an A-style subdivision of fns.c's hash-table section (or rename to make the fns.c lineage explicit); move builtin_mapatoms/builtin_unintern to the lread/obarray module; delete builtin_hash_table_keys/values |
| `interactive.rs` | callint.c, keyboard.c, keymap.c, data.c, eval.c, cmds.c | Split: call-interactively + interactive-spec parsing engine -> a callint.rs mirror (A); this-command-keys family -> the keyboard mirror; key-binding/where-is-internal/command-remapping/minor-mode-key-binding -> the keyma |
| `kmacro.rs` | macros.c + lisp/kmacro.el | Split: rename the macros.c core to macros.rs (A mirror); move kmacro-* functions to the lisp-shim population (ideally replaced by loading kmacro.el); review/drop the NEO_ONLY predicates or park them in the neomacs-only s |
| `load.rs` | lread.c, eval.c, fns.c | Split: load/load-path/lexical-cookie/eval_lisp_source_file → lread mirror (A_MISNAMED half); autoload/symbol-file → eval/lread mirrors; require/provide → fns mirror; the runtime-image/ldefs-boot/dump-surface sections (~2 |
| `lread.rs` | lread.c, keyboard.c, coding.c | Move read-event/read-char-exclusive to the keyboard mirror, read-coding-system/read-non-nil-coding-system to the coding mirror; merge the remaining lread.c pieces with the reader module to form the single lread.c mirror. |
| `marker.rs` | marker.c, editfns.c, data.c, alloc.c + lisp/subr.el (move-marker) | Keep the marker.c six here (A core); relocate point-*/mark-marker to the editfns mirror, markerp to data, make-marker to alloc; replace move-marker with the subr.el defalias when the shim layer allows. |
| `minibuffer.rs` | minibuf.c, keyboard.c + lisp/minibuffer.el (completion styles, read-file-name) | Split: minibuf.c subrs stay (A core); move recursive-edit/exit-/abort-recursive-edit/top-level into the keyboard mirror; classify the completion-style engine and read-file-name as lisp-shim (D) — long-term load minibuffe |
| `misc.rs` | fns.c, eval.c, keyboard.c, character.c, alloc.c | Disperse: fns.c items → fns mirror, backtrace family → eval mirror, recursion-depth → keyboard mirror, char conversions → character mirror, make-list → alloc mirror; retire misc.rs. |
| `navigation.rs` | editfns.c, cmds.c, syntax.c, fns.c + lisp/simple.el (transient-mark-mode) | Split: cmds.c five → a cmds.rs mirror (they are the whole of cmds.c's movement DEFUNs), editfns.c/fns.c items → editfns/fns mirrors, skip-chars → syntax mirror, transient-mark-mode → lisp-shim (load simple.el). |
| `process.rs (+ process/sys/)` | process.c, callproc.c, gnutls.c, print.c, fileio.c + subr.el, simple.el, obsolete/tls.el | Split: process.c core + process/sys/ stay as the process.c mirror (sys/ as its internal plumbing); call-process/call-process-region/getenv-internal -> new callproc.c mirror; gnutls_* -> new gnutls.c mirror; builtin_print |
| `reader.rs` | keyboard.c, minibuf.c, lread.c, fns.c, process.c + subr.el (read-number) | Split by GNU home: keyboard.c input fns + waiting-for-user-input-p wiring -> keyboard mirror; read-from-minibuffer/read-string/completing-read -> minibuf.c mirror; read/read-from-string/read-positioning-symbols -> lread  |
| `search.rs` | search.c + subr.el | Split: regexp-quote + string replace-match machinery -> the search.c mirror (merge with regex.rs per its A_MISNAMED recommendation); replace-regexp-in-string -> lisp-shim population (prefer loading subr.el). |
| `terminal/` | terminal.c, term.c | Split within population A: keep the terminal.c eight in terminal/ as the terminal.c mirror; move the eight tty-*/suspend-tty/resume-tty/controlling-tty-p subrs to a term.rs (term.c mirror) module. |
| `textprop.rs` | textprop.c (+foreign: buffer.c, xdisp.c) | Split: keep the 17 textprop.c subrs as the textprop.c mirror; move the 16 overlay subrs into the buffer.c mirror (or a buffer/overlays.rs sub-file of it); move get-display-property to the xdisp mirror. |
| `undo.rs` | undo.c + simple.el | Split: undo-boundary + undo-list/boundary machinery stay as the undo.c mirror (A); primitive-undo and undo are simple.el reimplementations that should be dropped in favor of loading lisp/simple.el (D). |
| `window_cmds/` | window.c, frame.c, term.c + window.el, frame.el | Split four ways: the 117 window.c subrs become the window.c mirror (A, window/ dir); the 50 frame.c subrs (all frame-*/make-frame/delete-frame/modify-frame-parameters) move to a frame.rs mirror; the 13 window.el + 2 fram |
| `xdisp.rs` | xdisp.c (+ window.c, keyboard.c, indent.c strays) | Keep the 12 xdisp.c subrs as the xdisp.c mirror (A); move move-to-window-line/pos-visible-in-window-p/window-line-height to the window.c mirror, posn-at-point/posn-at-x-y to keyboard, line-number-display-width to indent. |
| `encoding.rs` | coding.c, character.c, data.c, fns.c | Split: move the 4 coding subrs into the coding.c mirror (emacs_core/coding.rs), char-width/max-char into a character.c mirror, multibyte-string-p/char-or-string-p into the data.c mirror, string-bytes into the fns.c mirro |

### Machinery (B) — no elisp surface; keep out of mirror files
`buffer/`, `character/`, `compat_regressions.rs`, `data/`, `debug.rs`, `error.rs`, `face.rs`, `forward.rs`, `frontend_events.rs`, `gc_stats.rs`, `gc_trace.rs`, `heap_types.rs`, `hook_runtime.rs`, `image_catalog.rs`, `image_catalog.rs`, `intern.rs`, `jit/ and jit.rs`, `kbd.rs`, `logging.rs`, `mode.rs`, `network.rs`, `pdump/`, `perf_trace.rs`, `plist.rs`, `position.rs`, `prefix.rs`, `runtime_identity.rs`, `string_escape.rs`, `subr_docs/`, `symbol.rs`, `tagged/`, `test_utils.rs`, `tls.rs`, `treesit.rs`, `value.rs`, `value_reader.rs`, `var_docs/`, `wait.rs`, `window/`

## Runtime shadow probe: are the lisp-shims live divergences?

Probed `(type-of (symbol-function 'X))` for every EL-shadow name in both binaries after startup:

- **48 of 52 are OVERWRITTEN** — the preloaded `.el` (subr.el, window.el, simple.el, …) wins during loadup; the Rust subr is bootstrap scaffolding, not user-visible drift. The `should_install_public_subr` guard in `defsubr_with_entry` keeps it that way across pdump restores.
- **4 are fboundp-visible divergences**: `kmacro-add-counter`, `kmacro-set-counter`, `kmacro-set-format`, `open-tls-stream` — Rust subrs exist at startup where GNU keeps the name void until `(require 'kmacro)` / tls load. Fix: registration must move behind the same demand-load boundary (or be deleted with the shim).

Loadup parity: neomacs lisp/loadup.el loads the same preload set as GNU for every file relevant to el_shadow.txt: subr (line 125), version (128), international/mule (133), window (138), files (144), faces (160), international/mule-cmds (208), simple (251), frame (255), isearch (272), register (279). kmacro.el, bookmark.el, emacs-lisp/cl-lib.el and obsolete/tls.el all EXIST under /home/exec/Projects/github.com/eval-exec/neomacs-windows/lisp/ but are NOT in loadup.el — same as GNU (they are demand-loaded). Consequence verified at runtime: every Rust shim whose home .el is preloaded gets overwritten during loadup and shows the identical type as GNU (byte-code-function, or the same defalias for move-marker/not/string</string=/string>). The only divergence is the 4 names from non-preloaded libraries (kmacro-add-counter, kmacro-set-counter, kmacro-set-format, open-tls-stream): GNU batch startup has them VOID (not even autoloaded), while neomacs exposes live Rust subrs, so (fboundp ...) differs pre-load; after (require 'kmacro)/(require 'tls) both runtimes converge to byte-code-function, i.e. the Rust subrs do not survive the library load. Probe scripts/outputs: tmp/el_shadow_probe.el, tmp/el_shadow_probe2.el, tmp/el_shadow_neo.out, tmp/el_shadow_gnu.out. Note: both binaries need the nix develop shell (shared-library deps) to run.

## Registrar decomposition plan (`builtins::init_builtins`)

Corrected picture (the raw TSV's "508 bodies in mod.rs" was glob-import cross-talk): only **24 subr bodies** are truly defined in `builtins/mod.rs`; the rest live in `builtins/*.rs` (symbols.rs 102, treesit.rs 59, buffers.rs 52, stubs.rs 49, arithmetic.rs 34, types.rs 33, cons_list.rs 25, strings.rs 24, keymaps.rs 22, …). The decomposition target is the **flat ~8,300-line `init_builtins` fn (~1,620 registration calls)** → per-module `syms_of_*` functions.

Facts that make this safe (verified in source):

- Registration order among defsubrs is **not observable**: the global table is SymId-keyed with independent entries; arity/dispatch-kind come from `lookup_compat_subr_metadata` by name. `syms_of_*` order mirrors GNU `emacs.c` for cosmetics only.
- All defvar/symbol seeding happens in `Context::new_inner` **before** `init_builtins`; nothing inside interleaves with variables.
- Hard constraints: `symbols::init_event_symbol_properties` + `materialize_public_evaluator_function_cells` stay after ALL defsubrs; `pre_register_standard_fringe_bitmaps` after `init_builtins`; dump paths re-apply the dumped function surface after `init_builtins` (unchanged).
- The 3 duplicate registrations (`bare-symbol`, `markerp`, `nconc`) are byte-identical last-wins pairs — delete the earlier of each during decomposition.
- `init_builtins` was split out of `new_inner` because the combined frame broke debug codegen; decomposing into `syms_of_*` fns removes that pressure for good.
- Infrastructure staying in `builtins/mod.rs` (or moving to eval): dispatch shims, `BuiltinRegistration`/`register_builtin`, no-eval-policy tables, arg/number/string-char helpers, trace wrappers, thread-local resets, `from_value.rs` (pure typed-arg infrastructure), fringe bitmap state.
- Existing exemplars of the target pattern: `windows.rs::register_builtin_subrs` (cfg windows) and `builtins/lcms.rs::register_builtin_subrs`.

## Unported GNU DEFUNs: 245, zero core gaps

Triage: **169 platform-specific** (w32/haiku/android/x11-only), **42 niche**, **6 obsolete**, **0 core gaps**. Every DEFUN a normal Linux session can reach is ported — "100% elisp compatibility" holds at the name level; remaining risk is behavioral, which the oracle suite tracks.

## Neomacs-only surface audit (31 names)

23 are legit namespaced surface (`neomacs-*`, `neovm--*`) → `emacs_core/neo/`. The rest:

| name | finding | action |
|---|---|---|
| `defining-kbd-macro` | MISSED_MATCH: DEFVAR_KBOARD variable in src/macros.c:427 — a variable only, never a function i | Divergence: (fboundp 'defining-kbd-macro) is t in neomacs, nil in GNU; calling it as a function errors in GNU. Delete the function |
| `defining-kbd-macro-p` | INVENTION: none — GNU has no such symbol in any form; the idiom is reading the defining-kbd | Delete (callers should test the defining-kbd-macro variable, matching GNU), or rename to neomacs--defining-kbd-macro-p if internal |
| `executing-kbd-macro-p` | INVENTION: none — GNU only has the executing-kbd-macro DEFVAR_LISP variable (src/macros.c:4 | Delete (test the executing-kbd-macro variable instead, matching GNU), or rename to neomacs--executing-kbd-macro-p. Un-namespaced - |
| `transient-mark-mode` | MISSED_MATCH: define-minor-mode in lisp/simple.el:7614 (so it IS a function/command in GNU) pl | Exposing a function matches GNU's surface, but it must come from loading simple.el's define-minor-mode, not a Rust subr (navigatio |
| `treesit-language-version` | MISSED_MATCH: short-lived DEFUN in treesit.c (added 96d44c43217, Dec 2022) renamed to treesit- | Neomacs already registers treesit-language-abi-version as the real impl; this name is a pure alias (treesit.rs:1528). Delete the a |
| `treesit-parser-changed-ranges` | MISSED_MATCH: DEFUN added (996b9576713) then reverted (2ee3edce3f5); current GNU function is t | Neomacs already registers treesit-parser-changed-regions as the real impl; this name is a pure alias (treesit.rs:2758). Delete the |
| `x-scroll-bar-background` | INVENTION: none — GNU exposes this only as the frame parameter scroll-bar-background (src/f | Delete the 1-arg nil stub (builtins/stubs.rs:592) and route via (set-frame-parameter nil 'scroll-bar-background ...), or rename ne |
| `x-scroll-bar-foreground` | INVENTION: none — GNU exposes this only as the frame parameter scroll-bar-foreground (frame | Delete the 1-arg nil stub (builtins/stubs.rs:587) and route via set-frame-parameter with 'scroll-bar-foreground, or rename neomacs |

## Mechanisms

### Per-module `syms_of_*` (GNU pattern)

Each mirror module ends with its registrar; the central startup registrar
becomes an ordered list of `syms_of_*` calls (order preserved — startup order
is observable behavior). `windows.rs` and `builtins/lcms.rs` already follow
this shape.

### `defun!` — one definition point per subr

A `macro_rules!` macro colocating lisp name, typed Rust signature, and arity:
it expands to (a) a **typed public Rust fn** that internal code calls
directly — the analogue of GNU C calling `Fgoto_char` — and (b) the
`SubrDecl` used by `syms_of_*`. Docstrings and interactive specs deliberately
stay in the generated GNU-verbatim tables (`subr_docs/`, `interactive.rs`):
hand-copying them would reintroduce drift; verbatim extraction *is* the
parity guarantee.

### Arity mismatches must be loud

`defsubr_with_entry` currently lets `SUBR_ARITY_ORACLE` silently override the
declared arity. Once registrations are per-module and `defun!`-declared, a
mismatch between declared arity and the GNU-derived table is a bug in the
port and should fail at registration (debug assert), not be patched over.

## Migration status

| Step | Status |
|---|---|
| Extraction tooling + this document | done 2026-07-16 |
| Per-module `syms_of_*` decomposition of `builtins/mod.rs` | **done 2026-07-16** — 1,248 statements into 57 `syms_of_*` registrars; 316 statements remain in `init_builtins` (EL-shadow/NEO-only names, unmapped homes: `frame.c`, most of `emacs.c`, `fringe.c`, `cmds.c`, platform stubs, cfg blocks, `defsubr_pure!` units) |
| `emacs_core/shims/` quarantine | **done 2026-07-16** — abbrev, bookmark, cl_lib, isearch, rect, register moved with tracking headers; re-exported under old paths |
| `defun!` + exemplar migration | **done 2026-07-16** — `emacs_core/defun.rs`; exemplars: `zlib.rs` (full decompress.c mirror), `emacs.rs`, `identity`/`secure-hash-algorithms` in `fns.rs`. Note: `defun!`-declared subrs are invisible to the tmp TSV extractor and to the compat-surface `ctx.defsubr(` scanner — extend both parsers when converting `window.c`/`xfaces.c`-scope functions |
| Grab-bag dissolution (`misc.rs`, `builtins/misc_pure.rs`, …) | started — `emacs.c` cluster (daemonp, daemon-initialized, invocation-name/-directory) moved into new `emacs.rs` mirror; `identity` + `secure-hash-algorithms` into `fns.rs`. Remaining `misc_pure.rs` backlog by GNU home: message/message-box/message-or-box/current-message → `editfns.rs` (echo/message-log helpers split to `xdisp.rs`), prefix-numeric-value → callint mirror (`interactive.rs`), documentation-stringp → `doc.rs`, flush-standard-output → `print.rs`, force-mode-line-update → `buffer/`, get-internal-run-time → sysdep (no mirror yet), ignore → subr.el shim, make-symbol → `alloc.rs`, symbol-name → `data/` |
| Machinery eviction from `emacs_core/` (`jit/`, `perf_trace.rs`, …) | deferred (mechanical, low urgency) |
| Un-namespaced invention cleanup (see NEO-only audit) | deferred — needs per-name decision |

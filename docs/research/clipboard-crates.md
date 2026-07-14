# Rust clipboard backends for Neomacs

Research date: 2026-07-14

## Executive finding

Neomacs already intends to support bidirectional system clipboard text:
`kill-new`/copy reaches `interprogram-cut-function`, and yank/paste reaches
`interprogram-paste-function`. The GUI backend also exposes the Linux
`PRIMARY` selection. The implementation uses `arboard` 3.6.1, but its current
lifetime and Linux backend selection make the feature less reliable than the
Lisp interface suggests.

The pragmatic recommendation is:

1. retain `arboard` for Windows, macOS, and X11/XWayland;
2. use `smithay-clipboard` directly for a native Wayland window, because it
   uses the standard surface-oriented `wl_data_device` protocol;
3. hide both behind one long-lived, serialized `ClipboardService` owned by
   the display runtime; and
4. return or trace host errors instead of silently substituting a process-local
   cache.

`arboard` remains the best mature, general cross-platform crate in this
comparison. `clipboard-rs` has the richest API, but its native Wayland path is
new and also uses the data-control protocol intended for windowless utilities.
`copypasta` is mature but text-only, and its ordinary Linux alias still chooses
X11; its Wayland support is essentially a wrapper around `smithay-clipboard`.

## Neomacs behavior before this refactor

The GUI initialization installs Neomacs functions as GNU Emacs's
`interprogram-cut-function` and `interprogram-paste-function`
([`neo-win.el`](../../lisp/term/neo-win.el#L70-L82)). The selection methods
then map `CLIPBOARD` and Linux `PRIMARY` to text-only host calls
([`neo-win.el`](../../lisp/term/neo-win.el#L289-L362)). Therefore, ordinary
copy and paste are designed to work in both directions today.

At the investigation fixed point (`678bd28e`), the workspace requested
`arboard = "3.4"` and locked 3.6.1
([fixed-point `Cargo.toml`](https://github.com/eval-exec/neomacs/blob/678bd28e3dadbc43d2c0649c617a0c1d0f7de9cc/Cargo.toml#L149),
[fixed-point `Cargo.lock`](https://github.com/eval-exec/neomacs/blob/678bd28e3dadbc43d2c0649c617a0c1d0f7de9cc/Cargo.lock#L258-L277)). The lock entry had `x11rb`, but no
`wl-clipboard-rs`; consequently the optional native Wayland backend is not
compiled. On Linux that build used X11/XWayland.

Each operation created and immediately dropped a new `arboard::Clipboard`
([fixed-point `main.rs`](https://github.com/eval-exec/neomacs/blob/678bd28e3dadbc43d2c0649c617a0c1d0f7de9cc/neomacs-bin/src/main.rs#L996-L1042)). This conflicted with
arboard's Linux guidance: X11 and Wayland selection data is served by the
owning application, and long-running GUI applications should retain the
clipboard object in application state
([arboard ownership documentation](https://github.com/1Password/arboard/blob/d7c6cc971baca5e7d6fe45c3c28cd828f351558e/README.md#L40-L71)).

Finally, the VM builtins discarded host write errors, converted host read
errors to absence, and fell back to thread-local text
([fixed-point `stubs.rs`](https://github.com/eval-exec/neomacs/blob/678bd28e3dadbc43d2c0649c617a0c1d0f7de9cc/neovm-core/src/emacs_core/builtins/stubs.rs#L600-L668)).
That cache is useful for headless tests, but in a GUI it can make a failed
system copy appear successful and can return stale process-local text after a
failed system paste.

## Crate comparison

| Crate | Platforms and Linux backend | Data | Lifetime and threading | Assessment for Neomacs |
|---|---|---|---|---|
| [`arboard` 3.6.1](https://docs.rs/arboard/3.6.1/arboard/) | Windows, macOS, X11 by default; optional `wayland-data-control` prefers `ext-data-control`/`wlr-data-control` and falls back to X11 | Text, HTML, images, file lists | Synchronous API; `Clipboard` is `Send + Sync`, but parallel operations can fail, Windows parallel access can deadlock, and Linux ownership requires a long-lived instance | Best general default and already present. Serialize calls. Do not treat its optional data-control backend as the ideal native Wayland GUI path. |
| [`clipboard-rs` 0.3.5](https://docs.rs/crate/clipboard-rs/0.3.5) | Windows, macOS, X11; optional Wayland data-control selected from `WAYLAND_DISPLAY` | Text, HTML, RTF, PNG, files, arbitrary MIME, multi-format writes, change watching | Blocking synchronous operations; watcher should run on a separate thread; Wayland watcher polls | Broadest API and active in 2026, but much more surface than Neomacs currently needs. Its Wayland backend is recent, uses data-control, and manually marks Linux contexts `Send`; not the conservative choice yet. |
| [`copypasta` 0.10.2](https://docs.rs/copypasta/0.10.2/copypasta/) | Windows, macOS, X11; Wayland must be constructed explicitly from a raw `wl_display` | Text only | Mutable synchronous trait. Wayland wraps `smithay-clipboard` in `Arc<Mutex<_>>`; the caller owns raw-display validity | Proven by Alacritty and close to Neomacs's current text scope, but adds little over using `smithay-clipboard` directly and a separate cross-platform backend. |
| [`smithay-clipboard` 0.7.3](https://docs.rs/smithay-clipboard/0.7.3/smithay_clipboard/) | Wayland only, using standard `wl_data_device` plus primary-selection protocols | Text only | Takes a valid `wl_display` pointer, owns a worker thread/event queue, and joins it on drop | Strongest native Wayland+winit fit for current text semantics; it is a specialized backend, not a cross-platform replacement. |

### `arboard`

Arboard's public API covers text, HTML, decoded images, and file lists
([API source](https://github.com/1Password/arboard/blob/d7c6cc971baca5e7d6fe45c3c28cd828f351558e/src/lib.rs#L85-L147),
[getters](https://github.com/1Password/arboard/blob/d7c6cc971baca5e7d6fe45c3c28cd828f351558e/src/lib.rs#L182-L207)).
It explicitly discusses winit shutdown and multiple-thread behavior, while
warning against concurrent Windows access
([lifetime/thread contract](https://github.com/1Password/arboard/blob/d7c6cc971baca5e7d6fe45c3c28cd828f351558e/src/lib.rs#L36-L68)).

Its optional Wayland feature is not transparent support for every compositor.
Arboard documents that it uses data-control extensions, that not all
compositors implement them, and that pure Wayland then fails
([Wayland limitations](https://github.com/1Password/arboard/blob/d7c6cc971baca5e7d6fe45c3c28cd828f351558e/README.md#L19-L38)).
The backend checks `WAYLAND_DISPLAY`, tries data-control, and otherwise falls
back to X11
([backend selection](https://github.com/1Password/arboard/blob/d7c6cc971baca5e7d6fe45c3c28cd828f351558e/src/platform/linux/mod.rs#L124-L150)).

### `clipboard-rs`

Clipboard-rs exposes the largest content model in this group: text, HTML,
RTF, images, files, custom formats, and monitoring
([public API](https://github.com/ChurchTao/clipboard-rs/blob/7b598dfaf7f2e1f9d9d0a441e114dc91e4663b31/src/lib.rs)).
Its 0.3.5 source has a non-default Wayland feature and runtime fallback to X11
([features](https://github.com/ChurchTao/clipboard-rs/blob/7b598dfaf7f2e1f9d9d0a441e114dc91e4663b31/Cargo.toml),
[backend selection](https://github.com/ChurchTao/clipboard-rs/blob/7b598dfaf7f2e1f9d9d0a441e114dc91e4663b31/src/platform/mod.rs)).
The Wayland watcher polls every 500 ms by default, and the Linux wrapper uses
manual `unsafe impl Send`
([Wayland implementation](https://github.com/ChurchTao/clipboard-rs/blob/7b598dfaf7f2e1f9d9d0a441e114dc91e4663b31/src/platform/wayland.rs)).

Those are not necessarily defects, but they make this a larger and newer
integration bet than Neomacs needs for reliable text copy/paste.

### `copypasta` and `smithay-clipboard`

Copypasta's portable trait only gets and sets `String`, and its normal Unix
`ClipboardContext` alias is X11 even with default Wayland features enabled
([platform aliases](https://github.com/alacritty/copypasta/blob/c429615bb13f9341676bdb23457a4360cfa9d9e3/src/lib.rs)).
Native Wayland construction is an unsafe function taking a raw display pointer
([Wayland wrapper](https://github.com/alacritty/copypasta/blob/c429615bb13f9341676bdb23457a4360cfa9d9e3/src/wayland_clipboard.rs)).

That wrapper delegates to Smithay Clipboard. Smithay's project explicitly
targets GUI/windowing applications such as winit
([project scope](https://github.com/Smithay/smithay-clipboard/blob/26c2f53f15f6bdc4f41a442d0ae2c2d63bbc617c/README.md#L4-L12)).
It builds an independent Wayland event queue on a worker thread, requires the
foreign display to remain valid for its full lifetime, and joins the worker at
drop
([lifetime implementation](https://github.com/Smithay/smithay-clipboard/blob/26c2f53f15f6bdc4f41a442d0ae2c2d63bbc617c/src/lib.rs#L19-L103)).
It uses the standard data-device and primary-selection managers and associates
copy operations with the focused seat's latest input serial
([protocol state](https://github.com/Smithay/smithay-clipboard/blob/26c2f53f15f6bdc4f41a442d0ae2c2d63bbc617c/src/state.rs#L109-L139)).
Its limitation is explicit: only three text MIME variants are accepted
([MIME list](https://github.com/Smithay/smithay-clipboard/blob/26c2f53f15f6bdc4f41a442d0ae2c2d63bbc617c/src/mime.rs#L1-L20)).

This protocol distinction matters. The maintainer of `wl-clipboard-rs`, used
by arboard and clipboard-rs for Wayland data-control, says the crate is for
terminal programs, clipboard managers, and other windowless utilities. It
directs windowed applications to `wl_data_device`/primary-selection, for
example through `smithay-clipboard`
([official guidance](https://github.com/YaLTeR/wl-clipboard-rs#readme)).

## Winit integration

Winit 0.30.13 does not provide a clipboard API. Its public scope and API are
window creation, event-loop management, input events, and raw window/display
handles
([winit documentation](https://docs.rs/winit/0.30.13/winit/)). A winit design
discussion still lists clipboard APIs as work needing MIME-type and callback
semantics decisions
([winit issue 3367](https://github.com/rust-windowing/winit/issues/3367#issue-2068493118)).

Therefore Neomacs needs a separate clipboard backend. Winit's Wayland raw
display handle is the appropriate integration point for `smithay-clipboard`;
the clipboard service must not outlive that display.

## Recommended Neomacs abstraction

Use one display-owned service, not calls to a crate scattered through Lisp or
rendering code:

```text
GNU selection semantics
    -> ClipboardRequest { selection, operation, formats }
    -> long-lived ClipboardService (serialized requests)
       -> Windows/macOS/X11: arboard
       -> native Wayland: smithay-clipboard
    -> ClipboardResult / explicit ClipboardError
```

The service should enforce these invariants:

- one backend instance lives from display initialization through orderly
  event-loop shutdown;
- operations are serialized even when callers come from different Neomacs
  threads;
- `CLIPBOARD` and `PRIMARY` are distinct typed selections, with unsupported
  selections reported explicitly;
- backend selection follows the actual winit display backend, not just
  environment-variable guesses;
- a system error is observable through tracing and the Lisp-facing result;
  any process-local fake clipboard is restricted to headless/test hosts;
- the first implementation keeps today's text semantics, while request/result
  types leave room for HTML, image bytes, file lists, and GNU `TARGETS` later.

The smallest reliable improvement is to retain and serialize the existing
arboard instance. The long-term correct Linux implementation adds the
surface-oriented Wayland backend rather than merely enabling arboard's
data-control feature.

## Implementation status

This refactor implements that recommendation for text selections. The display
runtime owns one serialized service; Winit's actual raw display handle selects
the native Wayland backend; all other supported window backends use arboard;
and GUI failures reach both tracing and Lisp instead of falling back to stale
thread-local text. The thread-local clipboard remains only when no display
host exists, which keeps batch tests deterministic.

One upstream limitation remains: smithay-clipboard 0.7.3 has no explicit
selection-disown operation and its store calls are asynchronous. On Wayland,
Neomacs therefore publishes an empty text selection when GNU asks it to disown
one. This preserves the observable text/selection-exists behavior, but a future
smithay-clipboard API should replace that approximation with protocol-level
disowning and acknowledged writes.

# GNU Emacs GUI Return/Tab event semantics

## Scope and source

Primary source inspected: the clean local GNU Emacs checkout at
`/home/exec/Projects/git.savannah.gnu.org/git/emacs`, commit
`382123e69e2c0cae39e44f9b72ca3674eaec2ad1` (2025-07-14).

Question: does GNU Emacs collapse GUI Return/Tab into ASCII 13/9 at input,
or preserve named events and apply ASCII compatibility later?

## Findings

### 1. The GUI backend preserves the native named key

In the PGTK backend, the GTK `keyval` is retained as `keysym`
(`src/pgtkterm.c:5338-5345`). The range from `GDK_KEY_BackSpace` through
`GDK_KEY_Escape` is classified with other special keysyms
(`src/pgtkterm.c:5389-5394`), and the backend emits a
`NON_ASCII_KEYSTROKE_EVENT` containing that keysym. The source explicitly says
that `make_lispy_event` will convert it to a symbolic key
(`src/pgtkterm.c:5460-5468`). Thus GUI Tab and Return do not enter the shared
input layer as ASCII 9 and 13.

The X11 backend implements the same boundary: it recognizes
`XK_BackSpace..XK_Escape` (`src/xterm.c:20554-20560`) and enqueues the original
keysym as `NON_ASCII_KEYSTROKE_EVENT` for later symbolic conversion
(`src/xterm.c:20649-20667`).

### 2. Shared conversion produces `tab` and `return` symbols

The shared function-key table uses offset `0xff00`; its entries assign keysym
`0xff09` the name `tab` and `0xff0d` the name `return`
(`src/keyboard.c:5461-5476`). The `NON_ASCII_KEYSTROKE_EVENT` branch selects
that table and calls `modify_event_symbol`, producing the Lisp event symbol
(with modifier decoration when applicable) (`src/keyboard.c:6316-6319`,
`src/keyboard.c:6350-6358`).

ASCII input is intentionally different. `ASCII_KEYSTROKE_EVENT` follows a
separate branch and returns a Lisp fixnum with `XSETFASTINT`
(`src/keyboard.c:6242-6252`, `src/keyboard.c:6280-6302`). Consequently:

- GUI named Tab is the symbol `tab`; ASCII TAB is integer 9.
- GUI named Return is the symbol `return`; ASCII RET/C-m is integer 13.

GNU's public key syntax documents the same distinction: `RET` is Return/C-m,
while `<return>` is a function key that can be bound separately
(`lisp/keymap.el:347-354`).

### 3. ASCII compatibility is a conditional fallback

GNU installs these mappings in `function-key-map`:

```text
[tab]    -> [9]
[return] -> [13]
```

The defining Lisp says that direct bindings for the function-key symbols
override these mappings (`lisp/simple.el:10675-10686`) and creates the
`tab -> ?\t` and `return -> ?\C-m` entries at
`lisp/simple.el:10693-10702`.

This override behavior is enforced inside `read_key_sequence`:

1. Sequence termination depends on the currently active minor, local, and
   global maps; `function-key-map` is checked only when a sequence has no other
   binding (`src/keyboard.c:10565-10583`).
2. The incoming event is first looked up with `follow_key` in the active maps
   (`src/keyboard.c:11223-11235`).
3. If that lookup produced a real binding, the function-key scan is advanced
   past the event without translating it (`src/keyboard.c:11412-11424`).
4. Only the unbound branch scans `local-function-key-map` (whose parent is
   `function-key-map`) and permits replacement
   (`src/keyboard.c:11425-11450`). The variable's own documentation states the
   same condition explicitly (`src/keyboard.c:13924-13951`).

Therefore `[return]`/`[tab]` bindings win. Only when they are unbound do the
fallbacks expose existing RET/TAB bindings through integers 13/9.

### 4. Reproduction against GNU Emacs

Using GNU Emacs 31.0.90 (`0ee48ac4df205e0d915946b5db00e73a0cd21ae0`),
feeding symbols through `unread-command-events` and calling
`read-key-sequence-vector` produced:

```text
symbol bound:       return -> [return]
symbol unbound:     return -> [13]
symbol bound:       tab    -> [tab]
symbol unbound:     tab    -> [9]
```

The unbound cases had active bindings for `[13]`/`[9]`; the bound cases had
active bindings for `[return]`/`[tab]`. This directly exercises the lookup
ordering described by the source.

## Conclusion for Neomacs

GNU Emacs's invariant is:

```text
physical GUI named key
  -> symbolic Lisp event (`return` / `tab`)
  -> ordinary active-map lookup
  -> only if unbound, function-key fallback to 13 / 9
```

Accordingly, removing Neomacs's Return/Tab-to-integer conversion at GUI event
ingress matches GNU Emacs. It restores information required for separate
`[return]` and `[tab]` bindings while retaining RET/TAB compatibility through
the already-existing `function-key-map` fallback. No package-specific handling
is indicated by GNU's architecture.

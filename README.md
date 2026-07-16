<p align="center">
  <img src="assets/banner.svg" alt="NEOMACS banner"/>
</p>

<h3 align="center">GNU Emacs, rebuilt in Rust.</h3>
<p align="center">GPU rendering &nbsp;·&nbsp; JIT-compiled Elisp &nbsp;·&nbsp; low-pause GC &nbsp;·&nbsp; 100% Emacs compatibility</p>

<p align="center">
  <a href="https://github.com/eval-exec/neomacs/actions/workflows/ci.yml"><img src="https://github.com/eval-exec/neomacs/actions/workflows/ci.yml/badge.svg" alt="CI"/></a>
  <a href="#status--roadmap"><img src="https://img.shields.io/badge/status-alpha-blueviolet" alt="Status: Alpha"/></a>
  <a href="COPYING"><img src="https://img.shields.io/badge/license-GPL--3.0-blue" alt="License: GPL-3.0"/></a>
  <a href="https://github.com/eval-exec/neomacs/discussions"><img src="https://img.shields.io/badge/chat-discussions-brightgreen" alt="Discussions"/></a>
  <a href="https://github.com/sponsors/eval-exec"><img src="https://img.shields.io/badge/%E2%9D%A4-sponsor-db61a2" alt="Sponsor"/></a>
</p>

https://github.com/user-attachments/assets/85b7ee7b-3f4a-4cd2-a84f-86a91d052f11

**Neomacs** is a ground-up rewrite of GNU Emacs in Rust with one non-negotiable
rule: **your config, your packages, and your muscle memory just work.**
Compatibility is not a promise here — it is a test suite. Every rewritten
subsystem is verified by tens of thousands of differential tests that execute
the same Elisp in Neomacs and in a real GNU Emacs binary and demand identical
behavior.

## Highlights

- 🎨 **GPU display engine** — wgpu (Vulkan/Metal/DX12/GL) replaces `xdisp.c`; text, images, 4K video, WebKit views, and terminals render *inside buffers*, zero-copy via DMA-BUF
- ⚡ **JIT-compiled Elisp** — tiered Cranelift JIT, enabled by default, up to 12.8× on hot Elisp
- 🧹 **Low-pause GC** — incremental collector with sub-100µs pause slices by default; fully concurrent collector available behind a flag
- ✅ **Compatibility you can measure** — ~1,500 C primitives ported; a 37,000-test oracle suite diffs Neomacs against real GNU Emacs on every change
- 📦 **Real configs boot today** — starts Doom Emacs with 166 packages; `.el`/`.elc` load paths, pdump, batch mode, and the byte-compiler all work
- ✨ **Animations at refresh rate** — smooth cursor, scroll, and buffer-switch effects on the render thread, [fully configurable from Elisp](docs/animations.md)
- 🖥️ **GUI and TUI** — the same core drives the wgpu GUI and a terminal frontend

## Showcase

🎬 **[Watch the demo video](https://youtu.be/WZRWWuuNZX0)**

<img width="1447" alt="Inline 4K images in an Emacs buffer" src="https://github.com/user-attachments/assets/325719dc-dac4-4bd8-8fd9-e638450a489f" />

<details>
<summary><b>More:</b> inline WebKit browser · GPU terminal · rounded box faces · 4K video playback</summary>

<img width="1851" alt="Inline WPE WebKit browser in an Emacs buffer" src="https://github.com/user-attachments/assets/10e833ca-34b2-4200-b368-09f7510f50d0" />

<img width="1448" alt="Inline Alacritty terminal in an Emacs buffer" src="https://github.com/user-attachments/assets/175ffd75-78b5-46c9-9562-61cfd705e358" />

<img width="1868" alt="Rounded-corner box face attribute" src="https://github.com/user-attachments/assets/65db32f0-8852-4091-bd99-d61f839e0c95" />

https://github.com/user-attachments/assets/275c6d9a-fced-44f6-8f43-3bbd2984d672

</details>

## Quick start

```bash
git clone https://github.com/eval-exec/neomacs && cd neomacs

# Recommended: the dev shell provides every dependency (with a binary cache)
nix develop --accept-flake-config

# Compile Rust, bootstrap Elisp, generate the dump, run
cargo xtask fresh-build --release
./target/release/neomacs
```

No Nix? See **[docs/install.md](docs/install.md)** for Arch Linux and macOS
instructions, plus how to run the test suites.

## How it works

GNU Emacs is ~300,000 lines of single-threaded C, with a display engine
designed for 1980s terminals. Neomacs replaces that substrate while keeping
the surface: the Elisp evaluator, bytecode VM, JIT, GC, reader, and the
buffer/window/frame/process subsystems are Rust (`neovm-core`), and redisplay
runs as a Rust layout engine feeding a wgpu renderer on its own thread.

Neomacs began as a hard fork so the original C tree stays in-repo as the
**reference implementation and test oracle**: each rewritten subsystem is
diffed against real GNU Emacs behavior, form by form, before it ships. See
**[docs/architecture.md](docs/architecture.md)** for the module layout and
threading model, and
[docs/elisp-core-analysis.md](docs/elisp-core-analysis.md) for why the C core
is hard to evolve in place.

## Status & roadmap

> Neomacs is **alpha**: daily-drivable for the adventurous, with rough edges.
> Bug reports with a repro are gold — [file them here](https://github.com/eval-exec/neomacs/issues).

| Area | State |
|---|---|
| Rust display engine (layout + wgpu renderer, GUI) | ✅ done — `xdisp.c` fully bypassed |
| Rich media: images, 4K video, WebKit, terminal in buffers | ✅ done |
| Elisp runtime in Rust (evaluator, VM, GC, reader, subsystems) | ✅ running everything today; parity hardening ongoing |
| Tiered JIT for Elisp | ✅ shipped, on by default |
| TUI (terminal) frontend | 🔨 in progress |
| macOS | 🔨 experimental ([#22](https://github.com/eval-exec/neomacs/issues/22)) |
| Windows | 🔨 builds in CI; runtime bring-up in progress |
| True multi-threaded Elisp | 🗺️ research phase |

## FAQ

**Will my config and packages work?**
That is the point of the project. Neomacs loads your real `init.el` and real
packages — it boots Doom Emacs (166 packages) today. Anything that behaves
differently from GNU Emacs is a bug; please report it.

**Is this a fork of GNU Emacs?**
A hard fork, from commit [`705c0e3729`](https://git.savannah.gnu.org/cgit/emacs.git/commit/?id=705c0e3729bf53db9e84ae7c8b932ebc3b2da934),
synced up to [`0ee48ac4df2`](https://git.savannah.gnu.org/cgit/emacs.git/commit/?id=0ee48ac4df2) (`emacs-31.0.90`).
The changes are too invasive to be upstreamed, so the repo does not carry the
original git history; open an issue if you need it restored for reference.

**How is "100% compatibility" enforced, not just claimed?**
By construction: the C sources stay in the tree as an executable oracle. A
37,000-test differential suite runs identical Elisp forms in Neomacs and in a
real GNU Emacs binary and compares results, and subsystem ports are reviewed
against the corresponding C file. Divergences are tracked and burned down
like defects.

**Why Rust, and why not patch GNU Emacs instead?**
Memory safety across a 300k-line C codebase, a thread-safe foundation for a
future parallel Elisp, and a GPU rendering stack are substrate changes —
they need a new substrate. See [docs/architecture.md](docs/architecture.md).

## Contributing

Contributions are very welcome — the codebase is unusually well-tested, so
it's a safe place to hack:

- **Emacs users** — run your config, [report divergences](https://github.com/eval-exec/neomacs/issues)
- **Rust developers** — core subsystem ports, performance, GC/JIT
- **Graphics programmers** — shader effects, renderer optimizations
- **Writers** — docs, tutorials, examples

Discussion happens in [GitHub Discussions](https://github.com/eval-exec/neomacs/discussions).

## Sponsor

Neomacs is a long-term rewrite that takes sustained work to build, test, and
maintain. If it's useful or exciting to you, consider
[❤️ sponsoring its development](https://github.com/sponsors/eval-exec).

## Acknowledgments

Built with [wgpu](https://wgpu.rs/), [winit](https://github.com/rust-windowing/winit),
[cosmic-text](https://github.com/pop-os/cosmic-text),
[GStreamer](https://gstreamer.freedesktop.org/),
[ash](https://github.com/ash-rs/ash), and cursor animations inspired by
[Neovide](https://neovide.dev/).

## License

[GPL-3.0](COPYING) — the same license as GNU Emacs.

## Star history

[![Star History Chart](https://api.star-history.com/image?repos=eval-exec/neomacs&type=date&legend=top-left)](https://www.star-history.com/?repos=eval-exec%2Fneomacs&type=date&legend=top-left)

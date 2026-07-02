# Neomacs vs GNU Emacs Module Audits

**Date**: 2026-03-28

This directory now has one audit file per major compatibility module.

## Display Stack Performance & Architecture Audit (2026-07-02)

A six-part audit of the Rust frontend (architecture, GPU pipeline, performance),
with ranked findings and a modernization roadmap:

- [00 — Overview, verdict, ranked findings](2026-07-02-display-audit-00-overview.md)
- [01 — Runtime & Threading (neomacs-display-runtime)](2026-07-02-display-audit-01-runtime-threading.md)
- [02 — GPU Renderer (neomacs-renderer-wgpu)](2026-07-02-display-audit-02-gpu-renderer.md)
- [03 — Layout Engine (neomacs-layout-engine)](2026-07-02-display-audit-03-layout-engine.md)
- [04 — Protocol & Integration (neomacs-display-protocol, neomacs-bin, neovm-core)](2026-07-02-display-audit-04-protocol-integration.md)
- [05 — Modernization Roadmap (Phases 0–4)](2026-07-02-display-audit-05-modernization-roadmap.md)

- [Phase 1: Lisp VM Core](phase-01-lisp-vm-core.md)
- [Phase 2: Buffer & Text](phase-02-buffer-and-text.md)
- [Phase 3: I18n / Character / Coding](phase-03-i18n-character-coding.md)
- [Phase 4: Search / Read / Print / File I/O](phase-04-search-read-print-file-io.md)
- [Phase 5: Editing Commands](phase-05-editing-commands.md)
- [Phase 6: Window / Frame / Font / Terminal](phase-06-window-frame-font-terminal.md)
- [Phase 7: Display Engine](phase-07-display-engine.md)
- [Phase 8: Command System](phase-08-command-system.md)
- [Phase 8 Keymap/Key Input Plan](phase-08-keymap-key-input-refactor-plan.md)
- [Phase 9: Process / Thread / Timer](phase-09-process-thread-timer.md)
- [Process / Timer / Event Loop Audit](process-timer-event-loop.md)
- [Phase 10: Startup & Integration](phase-10-startup-integration.md)

Existing overview files remain important:

- [Compatibility Audit Sequence](neomacs-gnu-compatibility-audit-sequence.md)
- [Bootstrap Pipeline](bootstrap-pipeline-gnu-vs-neomacs.md)
- [GNU Backend Coupling Map](gnu-emacs-backend-module-dependencies.md)
- [Error / Condition System Audit](error-condition-system.md)
- [VM Harness Builtin Surface Audit](vm-harness-builtin-surface.md)
- [Thread Model Audit](thread-model-vs-gnu-emacs.md)

These audits are not a claim that Neomacs already matches GNU Emacs. They are
the current audit result and the required direction to make Neomacs 100%
semantically identical to GNU Emacs.

Each phase file is intended to stay source-code-level:

- which GNU source files own the semantics
- which Neomacs source files own them today
- where Neomacs ownership is split or architecturally wrong
- what the long-term ideal ownership should be
- what exit criteria would justify calling the phase GNU-compatible

Important scope note:

- Phase 1 is about GNU-compatible Lisp-visible semantics, not copying GNU
  Emacs's internal VM architecture.
- Phases 2 and above should stay much closer to GNU ownership and behavior,
  because that is where "load the same GNU Lisp and preserve the same editor
  semantics" matters most.

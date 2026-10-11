# Implementation status

October 11 local checkpoint records integrated Studio and editor repairs, cleanup and documentation consolidation. [Resume state and validation](IMPLEMENTATION_RESUME.md) is authoritative; no push or new remote CI is claimed.

| Area | Implemented |
|---|---|
| Published foundation | Shared production compiler/core, widths/strings/bank limits/provenance, world/collision/scroll, sprite/pool/animation, audio/SFX/DPCM, cartridge/battery/CPU/bus contracts; PR162 host/WASM CI passed |
| Studio | Design brief, stable scenes, editable art/audio starter, protected generated source, independent preview/test harness, entity placement/presets and RAM/OAM limits |
| Game rules/testing | Persistent WORD variables, ordered entry/button/timer/update rules, conditions, bounded state/scene/audio actions, visit latches and saved scene/position/entity/variable assertions |
| Editor correctness | Retryable WASM initialization, stale work cancellation, cache revalidation, visible PPU/Full controls, source protection, immediate input capture and actionable errors |

Local 350-test matrix and browser behavior are recorded in [resume](IMPLEMENTATION_RESUME.md) and [browser evidence](BROWSER_ACCEPTANCE_2026-10-10.md). [Runtime contracts](RUNTIME_CONTRACTS.md) define limits; [worksheet](PHASE_40_ACCEPTANCE_CHECKLIST.md) retains incomplete release gates.

Next: combat/spawners/hitboxes, reusable events/dialog/quests, inventory/save, asset/audio reuse, custom-main tests and profiling. [Studio roadmap](GAME_STUDIO_ROADMAP.md) separates current work from later [1980s ROM import](ROM_IMPORT_ROADMAP.md). The platform is still in development.

Cleanup removes obsolete test artifacts and duplicated notes, retains current projects/evidence/build dependencies, and excludes generated files from the commit. Exact changes and deletion manifest remain in ignored `.tools/notes`.

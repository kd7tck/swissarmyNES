# Development checkpoint — October 11, 2026

The user authorized temporary cleanup, compact documentation and a local commit. No push requested. `main` is the only local branch. Use `git log -1 --oneline` for the checkpoint identity; earlier published baseline178f1de/PR162 passed CI38065116804/38065320107 before this development.

## Implemented and verified

Studio now integrates scenes/assets, reusable decoration/pickup/door/patrol objects, placement, WORD variables, ordered conditions/actions and controller assertions. Generated virtual game_flow.swiss compiles through the production compiler/core, keeps source provenance and preserves custom main.swiss. Editor lifecycle/cache/read-only/save races and palette/selection/variable-label defects have regression coverage.

| Local check | Result |
|---|---|
| Native workspace/all-target tests |267 passed |
| Source frontend tests |80 passed |
| Generated production WASM tests |3 passed |
| Format / strict Clippy / JS syntax / server build | Passed |
| Browser | Persistence/rendering/read-only code; failed/repaired assertions;107-frame door round trip;70-frame score/revisit; timer-driven WASM room change |

These350 tests are bounded evidence, not full platform/phase acceptance. Current remote host-matrix CI, physical controllers, audio fidelity and custom-main execution remain separate gates. [Browser observations](BROWSER_ACCEPTANCE_2026-10-10.md) · [worksheet](PHASE_40_ACCEPTANCE_CHECKLIST.md).

## Resume safely

1. Read AGENTS, git status and ignored `.tools/notes/game-studio-2026-10-10.md`. It records all file changes, retained evidence, cleanup manifest and commit status. Preserve projects and source.
2. Use [README setup/checks](../README.md#setup). The copied acceptance server was stopped/removed during cleanup; start the normal server when needed. Active patched core, wasm-bindgen tools, build cache and generated WASM remain local/ignored.
3. Preserve protected gf base/entity declaration order; variable/event state follows it. Fixed addresses apply only to the isolated harness. Custom code uses generated names.
4. Continue enemy combat/spawners and configurable hitboxes, then reusable events/functions, dialog/quests, inventory/checkpoint/save and art/audio reuse/profiling. [Roadmap](GAME_STUDIO_ROADMAP.md).
5. Keep recovery notes current and run affected checks after implementation. Existing green results need repetition when new code, failures or concerns justify it. Publishing requires explicit authorization.

Retained local projects: platform-demo, studio-acceptance-20261010, studio-controls-20261011, studio-entities-20261011, studio-events-20261011. Event project is restored to Title/Score=0/pressed A; preview is closed. The pickup counter persists between rooms, so once-per-visit pickup conditions can fire immediately on return.

[Studio workflow/schema](GAME_STUDIO.md) · [runtime contracts](RUNTIME_CONTRACTS.md) · [historical handoff](DEVELOPER_HANDOFF_THROUGH_PHASE_40.md)

Later: [1980s NES/Famicom ROM recovery](ROM_IMPORT_ROADMAP.md), DESIGN Phase 48. Planned only; exact source reconstruction precedes semantic recovery, preserving original engine/mapper and uncertain regions.

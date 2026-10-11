# AI agent guidelines

## Rules

1. Read the Brain, [resume state](docs/IMPLEMENTATION_RESUME.md) and [DESIGN](DESIGN.md). Inspect implementation before treating a roadmap item as missing. User instructions take precedence over historical plans.
2. Keep Rust/Axum, vanilla HTML/JS/CSS and the shared production NES core. Preserve Windows/macOS/Linux/WASM compatibility. Do not add a separate validation CPU or change architecture without authorization.
3. Document **every file change** in ignored `.tools/notes/game-studio-2026-10-10.md` as work proceeds; compact redundant notes. Update README, DESIGN and this Brain for major changes. Preserve accurate code comments and explain why new code is needed.
4. Write a short plan, implement, verify actual behavior, then update recovery state. Add meaningful feature/bug tests; do not weaken assertions to hide failures. Formatting and strict Clippy must pass. Use production compilation/emulation for runtime evidence; mocks and assembly text alone cannot establish acceptance.
5. Full source checks: `cargo check --workspace --all-targets --locked`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`. `node scripts/verify.mjs` also prepares dependencies/fixtures, builds WASM and runs JS tests. Avoid concurrent core preparation/Cargo jobs; repeat checks when changes or failures warrant them.
6. Commit only when authorized and checks pass. Deliver source/build files, tests, docs and notices; exclude compiled binaries/WASM/ROMs, logs, screenshots, `.tools` and local projects. Preserve user source and credentials. A local commit does not authorize a push.

## Technical ownership

| Path | Responsibility |
|---|---|
| `src/compiler` | Language/analysis, 6502 codegen, assembly, audio and source mapping |
| `src/server` | Projects, protected virtual Studio source, API compilation/testing |
| `static` | Vanilla editors/Studio, emulator lifecycle and debugging |
| `emulator` | Checksum-pinned patched TetaNES 0.12.2 wrapper and native/WASM tests |
| `scripts` / `patches` | Reproducible dependency, fixture and output preparation |
| `examples` / `tests` | Authored inputs and production integration/regression tests |

NES RAM is 2 KiB; compiler variables are bounded separately from reserved runtime state. Use symbols and [runtime contracts](docs/RUNTIME_CONTRACTS.md), not assumed addresses. rs6502 needs supported data syntax (`db`/injection); `ELSEIF` is unavailable. Frontend has no bundler step.

## Brain — October 11 checkpoint

- User authorized temporary-file cleanup, documentation condensation and a **local commit**. No push requested; only local `main`. Previous no-commit/credit-stop instructions are superseded.
- Studio scenes/entities/variables/events generate ordinary SwissBASIC; virtual `game_flow.swiss` and custom `main.swiss` stay protected. Preview/tests use an isolated harness, excluding custom-main execution.
- Entities share room slots and fixed 16×16 hitboxes. Pickups respawn on entry; counter and WORD variables persist between rooms/reset on GameFlowStart. Conservative OAM budget rejects more than 64 sprites.
- Events: entry/update/pressed/held/timer, AND conditions, ordered bounded state/scene/audio actions. Entry is deferred; visit latches reset on entry. Persistent pickup conditions can match immediately on return. Preserve base/entity declarations; variable/event storage follows them.
- Evidence: **267 native +80 frontend +3 WASM**, strict fmt/Clippy/build, browser persistence/rendering/read-only code, failed/repaired assertions,107-frame door round trip,70-frame score/revisit and timer-driven WASM room change. [Browser summary](docs/BROWSER_ACCEPTANCE_2026-10-10.md). CI38065116804/38065320107 covers earlier PR162/main 178f1de, not this checkpoint.
- Cleanup stops the copied acceptance server and retains projects/current evidence/build tools. Exact deleted paths, remaining files and commit state are in ignored notes. Reinspect process/browser state before reuse.
- Next: enemy combat/spawners, configurable hitboxes, reusable event groups/functions, dialog/quests, inventory/checkpoint/save, art/audio reuse and ROM/scanline profiling. Physical controls, audio fidelity, custom-main testing and release/phase acceptance remain separate gates.
- Later: [1980s NES/Famicom ROM import](docs/ROM_IMPORT_ROADMAP.md), Phase 48. Exact reconstruction precedes semantic recovery; preserve engine/mapper and uncertain bytes. Importer not implemented; no original-source/every-emulator guarantee.

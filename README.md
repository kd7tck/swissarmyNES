# SwissArmyNES

SwissArmyNES is a Rust/Axum and vanilla JavaScript platform for NES games: code, graphics, maps, animation, music/SFX, game flow, testing and debugging.

## Current checkpoint — October 11, 2026

Game Studio connects scenes and assets, reusable pickup/door/patrol entities, persistent game variables and ordered event rules. Preview and saved controller scenarios run generated SwissBASIC through the production NES core while preserving custom source. See [Studio usage](docs/GAME_STUDIO.md), [next work](docs/IMPLEMENTATION_RESUME.md) and [roadmap](docs/GAME_STUDIO_ROADMAP.md).

Local verification passed **267 native + 80 frontend + 3 WASM tests (350)**, strict Clippy, formatting and server build. Browser acceptance covers persistence, entity rendering, score/room rules and failed/repaired assertions. Published baseline `178f1de` passed Linux/Windows/macOS/WASM CI; that run predates this checkpoint. [Evidence](docs/BROWSER_ACCEPTANCE_2026-10-10.md) and the [phase-40 worksheet](docs/PHASE_40_ACCEPTANCE_CHECKLIST.md) identify remaining release gates.

Later: [import original 1980s NES/Famicom ROMs into editable projects](docs/ROM_IMPORT_ROADMAP.md). The importer is planned, not implemented.

## Tools and language

| Area | Available functionality |
|---|---|
| SwissBASIC | BYTE/INT/WORD/STRING, arrays/records/enums, macros, includes, 16-bit arithmetic, IF/SELECT/FOR/WHILE/DO, PEEK/POKE, inline assembly and interrupts |
| Art/world | Palettes, CHR/PNG import, nametables, metatiles, world maps, metasprites and animations |
| Audio | Music tracker, volume/pitch/duty/arpeggio envelopes, SFX and WAV-to-DPCM import |
| Studio | Scene graph, map/spawn/audio bindings, entity placement, variables, events and controller/state assertions |
| Emulator/debug | Shared native/WASM core, keyboard/gamepad input, pause/reset/scaling/volume, source maps/breakpoints, RAM and PPU views |

`ELSEIF` is unsupported; nest IF/ELSE blocks. The compiler produces MMC1 images with eight 16 KiB PRG banks and 8 KiB CHR. ROM/RAM/asset limits are in [runtime contracts](docs/RUNTIME_CONTRACTS.md). Existing functionality does not imply full phase or cartridge acceptance.

## Setup

Requires Rust **1.98.0**, Node **22+**, Git, tar and the platform linker (Windows: MSVC C++ build tools).

```sh
git clone https://github.com/kd7tck/swissarmyNES.git
cd swissarmyNES
rustup toolchain install 1.98.0 --profile minimal --component rustfmt --component clippy --target wasm32-unknown-unknown
node scripts/prepare-core.mjs
cargo +1.98.0 install wasm-bindgen-cli --version 0.2.106 --locked
cargo +1.98.0 build -p swiss-emulator --release --target wasm32-unknown-unknown --locked
wasm-bindgen --target web --out-dir static/wasm --out-name swiss_emulator target/wasm32-unknown-unknown/release/swiss_emulator.wasm
cargo +1.98.0 run --locked
```

Open **http://127.0.0.1:3000** and create a playable starter in Game Studio. Reload after editor changes; rebuild WASM after emulator changes. Preparation verifies and patches TetaNES 0.12.2 into ignored `.tools/tetanes-core`. Run it on fresh checkouts or after patch changes, with no concurrent Cargo jobs. [Patch policy](patches/README.md).

## Verification

```sh
node scripts/verify.mjs
```

This prepares pinned fixtures/core, checks formatting, runs native tests/strict Clippy, generates WASM and runs JavaScript tests. Options: `--native`, `--frontend`, `--offline` (Cargo dependencies must be cached; missing fixture/core inputs may still require downloads). The verifier also finds wasm-bindgen in `.tools/bin`. [Fixture provenance](emulator/tests/fixtures/PROVENANCE.md).

Install an editable example with `node examples/platform-demo/install.mjs`; [example instructions](examples/platform-demo/README.md) describe controls and tests. Existing projects are preserved.

## Repository and delivery

`src/compiler` owns compilation; `src/server` owns projects/Studio/API; `static` contains the browser application; `emulator` wraps the production core. `tests` and `emulator/tests` exercise production behavior; `scripts` and `patches` reconstruct dependencies and outputs.

Commit source, build configuration/scripts, tests, authored example inputs, documentation and required notices. Generated binaries/WASM/ROMs, logs, screenshots, temporary files and local `projects` stay excluded. Cleanup retains current recovery notes/evidence and active build dependencies.

[Master roadmap](DESIGN.md) · [contributor instructions](AGENTS.md) · [phase-40 developer handoff](docs/DEVELOPER_HANDOFF_THROUGH_PHASE_40.md) · [historical audit](docs/BASELINE_AUDIT.md) · [emulator ownership](docs/EMULATOR_ARCHITECTURE.md)

Project license: [GNU GPL version 3](LICENSE). The prepared dependency retains upstream notices.

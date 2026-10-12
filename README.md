# SwissArmyNES

Rust/Axum and vanilla JavaScript tools for NES game code, graphics/worlds, music/SFX, Studio authoring, production emulation and debugging. SANBasic uses compatible `.swiss` files.

[DESIGN](docs/DESIGN.md) is the single merged development guide: [current state](docs/DESIGN.md#current-state), [plan](docs/DESIGN.md#plan), [Studio/schema](docs/DESIGN.md#studio-authoring), [runtime](docs/DESIGN.md#runtime-contracts), [acceptance](docs/DESIGN.md#acceptance-and-release) and [development record](docs/DESIGN.md#development-record). [AGENTS](AGENTS.md) owns contribution/recovery/publishing rules. Current development remains uncommitted.
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

[Master roadmap](docs/DESIGN.md) · [contributor instructions](AGENTS.md) · [phase-40 developer handoff](docs/DESIGN.md#acceptance-and-release) · [historical audit](docs/DESIGN.md#development-record) · [emulator ownership](docs/DESIGN.md#runtime-contracts)

Project license: [GNU GPL version 3](LICENSE). The prepared dependency retains upstream notices.

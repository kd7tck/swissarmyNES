# AI agent guidelines

## Rules

1. Read [current state](docs/DESIGN.md#current-state), the [work audit](docs/DESIGN.md#development-record) and [merged guide](docs/DESIGN.md). All plans, status, contracts and acceptance are sections of DESIGN; do not create duplicate docs. For each candidate identify existing modules/tests and classify implemented/pending acceptance/partial/missing before designing changes. Do not duplicate working code, tools or historical repairs. User instructions take precedence over historical plans.
2. Keep Rust/Axum, vanilla HTML/JS/CSS and the shared production NES core. Preserve Windows/macOS/Linux/WASM compatibility. Do not add a separate validation CPU or change architecture without authorization.
3. Document **every file change** in ignored `.tools/notes/game-studio-2026-10-10.md` as work proceeds; compact redundant notes and avoid old-data backup copies. Update DESIGN for state/contracts/planning; README only for setup changes. Preserve accurate code comments and explain why new code is needed.
4. Write a short plan, implement, verify actual behavior, then update recovery state. Add meaningful feature/bug tests; do not weaken assertions to hide failures. Formatting and strict Clippy must pass. Use production compilation/emulation for runtime evidence; mocks and assembly text alone cannot establish acceptance.
5. Full source checks: `cargo check --workspace --all-targets --locked`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`. `node scripts/verify.mjs` also prepares dependencies/fixtures, builds WASM and runs JS tests. Avoid concurrent core preparation/Cargo jobs; repeat checks when changes or failures warrant them.
6. Commit only when authorized and checks pass. Deliver source/build files, tests, docs and notices; exclude compiled binaries/WASM/ROMs, logs, screenshots, `.tools` and local projects. Preserve user source and credentials. Every future commit must include a very detailed commit body documenting substantive changes, their purpose, affected files/components, validation commands/results and material limitations. Put these notes in the Git commit itself. A local commit does not authorize a push. When publishing is requested, push a separate branch, open a pull request, wait for successful checks on its exact head, merge into `main`, then delete the merged temporary branch. Do not push development directly to `main`.

## Technical ownership

| Path | Responsibility |
|---|---|
| `src/compiler` | Language/analysis, 6502 codegen, assembly, audio and source mapping |
| `src/server` | Projects, protected virtual Studio source, API compilation/testing |
| `static` | Vanilla editors/Studio, emulator lifecycle and debugging |
| `emulator` | Checksum-pinned patched TetaNES 0.12.2 wrapper and native/WASM tests |
| `scripts` / `patches` | Reproducible dependency, fixture and output preparation |
| `examples` / `tests` | Authored inputs and production integration/regression tests |

NES RAM is 2 KiB; compiler variables are bounded separately from reserved runtime state. Use symbols and [runtime contracts](docs/DESIGN.md#runtime-contracts), not assumed addresses. rs6502 needs supported data syntax (`db`/injection); `ELSEIF` is unavailable. Frontend has no bundler step.

## Resume pointer

Latest user scope, verified results and next slice live only in [DESIGN Current state](docs/DESIGN.md#current-state). Recovery notes record every changed path. User authorized finishing development, committing all source/docs to a test branch, pushing/opening a PR, waiting for successful checks on that exact head, then merging main and deleting the temporary branch. Follow rule6; generated files and recovery data stay excluded.

# SwissArmyNES: developer handoff through phase 40

Prepared against commit `4610516840d7991bce7e1d879e1f4e2fa831f10e` (2026-09-12), retrieved from <https://github.com/kd7tck/swissarmyNES>. Audit performed September 12–13, 2026. This is an execution plan, not a claim that the listed repairs or phases have been completed.

**Outcome:** a developer can clone the project, build both Rust and browser artifacts, create/save/compile/run a game, use all phase 1–38 features, inspect cartridges and bus behavior, and demonstrate phase 40 CPU accuracy with a reproducible canonical nestest comparison.

**First task:** repair the production compilation path. At the audited commit, workspace tests pass while `POST /api/compile` returns HTTP 400 for even an empty `Main`. Do not start by implementing another CPU.

## Contents

1. [Scope and operating rules](#1-scope-and-operating-rules)
2. [Verified baseline and implementation map](#2-verified-baseline-and-implementation-map)
3. [Execution order and prerequisites](#3-execution-order-and-prerequisites)
4. [Repair the shared foundation](#4-repair-the-shared-foundation)
5. [Close phases 1–10](#5-close-phases-110-language-and-project-foundations)
6. [Close phases 11–20](#6-close-phases-1120-game-runtime)
7. [Close phases 21–30, including 25a–25d](#7-close-phases-2130-audio-and-visual-authoring)
8. [Close phases 31–38](#8-close-phases-3138-browser-emulator-and-debugging)
9. [Implement and prove phase 39](#9-phase-39-repository-ingestion-and-buscartridge-architecture)
10. [Implement and prove phase 40](#10-phase-40-ricoh-2a03-cpu-hardening)
11. [Release acceptance and handover](#11-release-acceptance-and-handover)
12. [References](#12-references)

Companion files: `docs/BASELINE_AUDIT.md` records evidence and limitations; `docs/PHASE_40_ACCEPTANCE_CHECKLIST.md` is the sign-off worksheet. Paths in this document are relative to the repository root unless explicitly described as dependency paths. Names marked **new** are proposed files or APIs to implement; they do not exist at the audit baseline.

## 1. Scope and operating rules

1. Read `AGENTS.md`, then `DESIGN.md`, then this handoff. Use current `DESIGN.md` phase numbering. Phase 39 is **Repository Ingestion & Bus/Cartridge Architecture**; phase 40 is **Ricoh 2A03 CPU Core Hardening**. The old Brain instruction “Start Phase 39: Mappers - MMC1” is stale.
2. Treat a “Completed” heading as historical status. Retain working implementation; repair or add only what its completion criteria and the concrete gaps below require. A compiler test that searches assembly text does not establish runtime correctness.
3. Keep Rust/Axum and the plain HTML/CSS/JavaScript frontend. Preserve Windows, macOS, Linux, and browser compatibility. Do not introduce egui, imgui, cpal, Tauri, or a frontend framework in this assignment; those are outside the requested phase boundary.
4. Use the existing TetaNES core as the starting architecture. Audit, expose, test, and patch it where necessary. `emulator/src/lib.rs` is a wrapper, not an in-repository CPU implementation. Dependencies can satisfy behavior, but their presence alone cannot satisfy phase acceptance.
5. Record the architecture in **new** `docs/architecture/emulator-core.md`: version, ownership of CPU/bus/mapper/cartridge state, extension points, and upstream patch strategy. If an actual core replacement is proposed, obtain a separate architecture decision before doing it; the default plan below does not require a replacement.
6. Phase 39's named `Mapper` abstraction must connect to execution. A trait that is only exercised by a mock while the browser uses unrelated behavior is not completion. Section 9 specifies how to handle the mismatch with TetaNES's existing mapper interfaces.
7. Preserve the existing compiler's mapper-1 output while restoring it. NROM support for test fixtures is a phase 39 prerequisite; a new compiler mapper-selection feature is not required. Do not silently change all generated games to mapper 0 to make nestest easier.
8. Do not broaden into phases 41–47. Preserve existing PPU/APU/mapper behavior inherited from TetaNES because phases 31–38 depend on it. New general MMC3 support, new PPU/APU implementations, save states, rewind, desktop packaging, and asset libraries are outside this handoff.
9. For each task, make a focused commit or PR with the failing reproduction, implementation, relevant automated tests, and manual evidence if applicable. Fix the assertion's underlying requirement; do not weaken tests or change a golden trace to match broken behavior.
10. Update the README, DESIGN status, and AGENTS Brain when a milestone actually passes. Preserve existing comments unless incorrect; correct stale comments around ROM size, memory ownership, generated artifacts, and roadmap numbering.

## 2. Verified baseline and implementation map

### 2.1 What was observed

| Item | Audit result | Consequence |
|---|---|---|
| Repository | Clean clone of the commit above | Recheck HEAD before implementing this plan |
| Rust environment | rustc 1.98.0, Cargo 1.98.0, Windows MSVC target | Other platforms and WASM still need CI proof |
| Workspace tests | `cargo test --workspace --all-targets --locked` passed | Existing coverage does not prove browser compilation |
| Emulator tests | `swiss-emulator` ran **zero tests** | Add tests around the actual emulator integration |
| Formatting | `cargo fmt --all -- --check` passed | Keep this gate |
| Strict Clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` passed | Native quality gate, not end-to-end acceptance |
| Production compile | Three direct API probes returned HTTP 400 assembler errors | Release blocker; see R02 |
| Source map | Rust pairs `(line,address)`; hover/breakpoints expect triples `(line,bank,address)`; active-line lookup still expects pairs | One versioned contract must replace the conflicting formats |
| Runtime generation | `generate_banks` omits helpers emitted by `generate` | Test and use the production path consistently |
| Bank state | `current_prg_bank` appears in generated instructions but no definition was found; debugger reads `$07F0` | Define/reserve compiler state and use mapper-derived debugger identity |
| ROM output | Assembler unconditionally emits 128 KiB PRG, 8 KiB CHR, mapper 1 | Existing tests have stale 40 KiB comments; protect mapper-1 compatibility |
| Project world assets | API injects only `nametables.first()`; world contains indices | Multiple screens are not yet proven to survive compile/runtime |
| WASM distribution | JS, declarations, and two `.wasm` files tracked under `static/wasm`; native Linux `emulator/wasm-pack` also tracked | Establish portable, reproducible artifact generation |
| Dependency cartridge parser | TetaNES 0.12.2 rejects trainers; inspected NES 2.0 path combines ROM size nibbles as bank counts | Close format gaps explicitly instead of assuming full NES 2.0 coverage |
| PPU palettes | Core writes a 16×2 RGBA palette image; wrapper exposes 4 KiB and UI treats it as one long row | Replace guessed buffer dimensions with a documented contract |

See the companion audit for exact API errors and the scope of verification. Source-derived risks in this plan require a reproducing test before being reported as a runtime failure.

### 2.2 Where to work

| Area | Existing files / symbols | Primary responsibilities |
|---|---|---|
| Language | `src/compiler/{lexer,parser,ast,preprocessor}.rs` | Tokens, grammar, include/macro expansion, source provenance |
| Semantics | `src/compiler/{analysis,symbol_table}.rs` | Types, declarations, bank metadata, identifiers |
| Code generation | `src/compiler/codegen.rs`: `generate`, `generate_banks`, `emit`, `estimate_size`, `allocate_memory` | Runtime helpers, placement, RAM allocation, source mapping |
| ROM assembly | `src/compiler/assembler.rs`: `assemble`, `assemble_banks` | CPU addresses to physical banks, segment checks, header and asset injection |
| Production compiler API | `src/server/api.rs`: `compile`, `compile_source` | Includes, assets, semantic analysis, bank generation, assembly, JSON |
| Project persistence | `src/server/project.rs`; `static/js/project.js` | Source files, metadata, asset schemas and round-trip preservation |
| Browser integration | `static/js/{app,editor}.js` | Save before compile, load WASM, input, timing, source maps, breakpoints |
| Visual editors | `static/js/{palette,chr,map,metatile,world,sprite}.js` | Author and serialize graphics assets |
| Audio | `src/compiler/audio.rs`; `static/js/{audio,sfx,envelopes,synth}.js` | Music, DPCM, envelopes, SFX authoring and encoding |
| Emulator wrapper | `emulator/src/lib.rs`, `emulator/Cargo.toml` | TetaNES integration, WASM exports, execution and debug access |
| PPU frontend | `static/js/ppu_viewer.js` | Pattern, nametable, palette, OAM rendering |
| Automation | `.github/workflows/rust.yml` | Currently Linux-only root-package checks |

Dependency source paths below mean the downloaded `tetanes-core-0.12.2/src/` directory, not files already owned by SwissArmyNES: `cart.rs`, `bus.rs`, `cpu.rs`, `cpu/`, `mapper.rs`, `mapper/`, `ppu.rs`, `control_deck.rs`.

## 3. Execution order and prerequisites

### 3.1 Milestones and dependencies

| Order | Work package | Must be complete first | Exit evidence |
|---|---|---|---|
| M0 | Capture baseline and reconcile docs | Clone | Reproduction commands and status inventory |
| M1 | R01–R05: CI, compile pipeline, placement, source maps, harnesses | M0 | Minimal API compile succeeds; ROM boots; harness is usable |
| M2 | Phases 1–10 acceptance and repairs | M1 | Language programs execute with expected RAM results |
| M3 | Phases 11–20 acceptance and repairs | M2 | Input/render/gameplay smoke project passes |
| M4 | Phases 21–30 acceptance and repairs | M3 | Assets persist, compile, and appear/play in emulator |
| M5 | Phases 31–38 integration hardening | M1–M4 | Fresh browser build passes debugging and emulator checks |
| M6 | Phase 39 architecture, parser, bus, mapper integration | M5 | Header and bus fixtures pass through production backend |
| M7 | Phase 40 CPU contract and nestest validation | M6 | Full pinned trace and supplemental CPU tests pass |
| M8 | Clean-checkout release acceptance and documentation | M7 | Completed sign-off worksheet and reproducible artifacts |

Build a small headless harness in M1 around the existing core so earlier phases can be executed. M7 strengthens that harness and establishes CPU correctness; it does not require waiting until M7 to run games. Repeat affected compiler runtime tests after CPU repairs.

### 3.2 Initial setup commands

Run from a terminal with Git, Rust stable, and the platform's Rust linker/toolchain available. On Windows use the MSVC C++ build tools. Use an installed host-native wasm-pack; do not execute the checked-in Linux binary on Windows/macOS.

```text
git clone https://github.com/kd7tck/swissarmyNES.git
cd swissarmyNES
git status --short
git rev-parse HEAD
rustc --version
cargo --version
cargo fetch --locked
cargo test --workspace --all-targets --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
rustup target add wasm32-unknown-unknown
```

For an existing checkout, inspect local changes first, then `git fetch origin` and `git pull --ff-only` on its intended branch. Never discard a developer's changes to reproduce this baseline. If HEAD differs, compare the relevant files and update this handoff's findings before acting on them.

Initial WASM build recipe, to verify and then codify in R01:

```text
cargo install wasm-pack --version 0.13.1 --locked
cargo check -p swiss-emulator --target wasm32-unknown-unknown --locked
wasm-pack build emulator --target web --out-dir ../static/wasm --out-name swiss_emulator
cargo run --locked
```

Open `http://127.0.0.1:3000`. Version 0.13.1 is the version recorded by the repository stamp, not a recommendation to float to the newest tool. Verify compatibility with the lockfile and select/pin a replacement only if this version cannot reproduce the build. Ensure the WASM build honors Cargo.lock; inspect the installed wasm-pack's argument support and document its locked-build option or use a locked Cargo build plus the matching wasm-bindgen CLI. Regeneration can change tracked artifacts; inspect that diff.

### 3.3 Evidence standard for every phase

1. Give each acceptance program an identifier, source/asset fixture, deterministic expected result, and maximum frames/instructions. A timeout is a failure, not a skip.
2. Call `compile_source` for integration tests and cover `POST /api/compile` separately. Keep lexer/codegen unit tests as smaller diagnostic tests.
3. Load the produced ROM in the same emulator backend used by the browser. Run to a known marker, frame count, or breakpoint. Read results without modifying emulated state.
4. For a UI feature, record create → edit → save → reload → compile → run. Check both data persistence and the produced game. A screenshot of the editor alone is insufficient.
5. Record automated command, result, platform, commit, and evidence location in the acceptance worksheet. For hardware input/audio tests, identify the browser/device used.
6. Mark a phase complete only when every required item passes; record any optional enhancement separately. Do not use a blanket “1–38 complete” without the per-phase evidence.

## 4. Repair the shared foundation

### R01 — Make build and test coverage represent the shipped application

**Files:** `.github/workflows/rust.yml`, both Cargo manifests, `README.md`, `AGENTS.md`; **new** build documentation and browser test harness.

1. Capture current commands and failures in `docs/BASELINE_AUDIT.md`. Keep the initial tests before introducing fixes.
2. Add Windows, Linux, and macOS native CI jobs using a chosen supported Rust version. Run workspace/all-target tests and Clippy, and all-workspace format checks. Keep `--locked` for reproducibility.
3. Add a separate WASM job installing the target and pinned generation tools, building `swiss-emulator`, and checking its exported API against frontend usage. A native `cdylib` compilation is not a WASM build test.
4. Decide whether generated `static/wasm` artifacts are committed or packaged in CI. The clean-clone startup instructions must work in either policy. If committed, verify a clean regeneration; if packaged, ensure the server/release bundle contains them.
5. Determine which of `swiss_emulator.wasm` and `swiss_emulator_bg.wasm` is loaded by generated JS. Retain only the required artifact in a cleanup commit after confirming all references. Remove the tracked host-specific build executable from the distribution after replacing it with an installation recipe; retain necessary notices.
6. Add a small browser automation suite with a pinned runner and lockfile. It may be development-only; retain the frontend's no-bundler runtime. Test page startup, project save, compile/run, source-map hover, and breakpoint pause.
7. Publish failing test output and screenshots as CI artifacts. Set explicit timeouts and avoid network access inside routine CPU/ROM test execution.

**Done:** all intended packages are covered, the browser loads freshly generated WASM, and a clean checkout has one documented development workflow on all three hosts.

### R02 — Restore the production Compile/Run pipeline

**Files:** `src/server/api.rs`, `src/compiler/codegen.rs`, `src/compiler/assembler.rs`; **new** `tests/compile_api_test.rs`.

1. Add an API regression test for `SUB Main()\nEND SUB`. Require HTTP 200, valid JSON, a decodable ROM, and a structurally valid source map. At baseline this fails with an assembler addressing error.
2. Add equivalent `compile_source` tests for WORD math, `Controller.Read()`, text output, one metasprite, DATA/READ, and asset-backed compilation. Include at least one named project/include case.
3. Capture the bank/address annotated assembly for the failing minimal program. Map the assembler's reported line to that listing. Investigate the undefined `current_prg_bank` reference and unsupported addressing syntax independently; the API error by itself is not proof of one root cause.
4. Make one shared runtime-emission path serve `generate` and `generate_banks`, or explicitly reduce `generate` to a compatibility wrapper. Ensure controller, text, sprite, animation, pool, collision, scroll, and user data emitters are available in production, with no duplicate helper labels.
5. Allocate and define runtime bank state in one memory-layout module. If `$07F0` remains the location, exclude it from all global/array/parameter allocations. Initial RAM clearing is not a substitute for initializing the mapper and bank shadow consistently.
6. Correct first-pass bank tracking: read `BANK` declarations while allocating/registering subroutine signatures, validate the bank range, and assign declarations to their intended banks. Do not use an unchanged `current_bank` for every signature.
7. Validate startup and vectors for mapper-1 power-on/reset conditions. Place any code required to establish a known bank in reliably mapped storage, then initialize MMC1 and transfer to Main. Test reset after a bank switch, not only initial load.
8. Define cross-bank call behavior, including nested calls, arguments, returned values/registers, and NMI interruption during serial mapper writes. Ensure fixed-bank trampolines can resolve symbols located in each switchable bank. Test forward references and same-address routines in different banks.
9. Execute all API-produced fixtures in the emulator and assert result memory after a bounded run. Add a browser Compile/Run smoke test with unsaved editor changes.

**Done:** the previously failing API cases compile and boot; every retained test path exercises shared production logic; no phase depends on an old-only helper path.

### R03 — Enforce ROM/RAM placement and deterministic output

**Files:** `src/compiler/{assembler,codegen,audio}.rs`; **new** memory-layout definitions and bank-layout tests.

1. Document each ROM segment's physical bank, CPU window, start/end, alignment, and owner. Use checked arithmetic before converting lengths to `u16` or `usize`.
2. Reject a switchable-bank segment that crosses `$BFFF` even if the resulting physical offset fits inside the complete 128 KiB vector. Reject fixed-bank segments beyond `$FFFF`, addresses below the legal window, invalid bank numbers, and injections that cross a bank boundary.
3. Apply the same checks to injected palettes, nametables, music, envelopes, samples, strings, and data pointers. Error messages must identify bank, requested range, allowed range, and conflicting owner.
4. Reserve vector bytes `$FFFA–$FFFF` and validate the data-pointer area before those bytes. Account for actual emitted table size, including user structures and strings.
5. Add checked RAM sizing for nested structures, arrays, parameters, string pointers, and runtime state. Test exactly-full allocation and one-byte overflow. Prevent pre-check arithmetic wrap or truncation of a large dimension.
6. Define supported asset sizes explicitly. Replace silent truncation with a diagnostic unless padding/truncation is a documented import operation. In particular, verify CHR input larger than 8 KiB and nametables larger/smaller than the expected 960+64 bytes.
7. Compile the same fixture repeatedly and compare ROM bytes and source maps. Sort unordered collections where they influence placement or label ordering.
8. Parse the resulting header and assert mapper 1, eight 16 KiB PRG banks, one 8 KiB CHR bank, and total length 139,280 bytes for the current output layout. Check vector-to-file offsets using the physical fixed bank.

**Done:** intentional limits produce clear errors and no asset/code segment can silently corrupt another bank or reserved RAM.

### R04 — Replace the conflicting source-map formats

**Files:** `src/compiler/{ast,preprocessor,codegen,assembler}.rs`, `src/server/api.rs`, `static/js/{app,editor}.js`; **new** source-map contract tests.

1. Define a versioned object format shared by Rust and JS. Suggested record: `{file, line, column?, bank, cpu_start, cpu_end, rom_offset?, kind}` with half-open address ranges. Define physical bank units and file identifiers; exclude the 16-byte header from a PRG-relative offset or explicitly name a file-relative offset.
2. Include a source revision/content identity in compile results. Reject or visibly invalidate stale breakpoints/highlights when editing or switching projects. Keep the external `{rom,map}` response keys if useful, but migrate every consumer together.
3. Preserve originating file and line through INCLUDE and macro expansion. For a macro choose a documented primary call-site location and optional definition location. Do not conflate line 10 of multiple files.
4. Prefer resolved assembler labels/listings for final addresses. If rs6502 exposes no usable listing, instrument its assembled segments/labels or introduce a verified mapping pass. Do not present an unchecked size estimate as an authoritative range.
5. Add regression cases for `LSR A`, `ASL A`, one/multiple `db` values beginning with `$`, multi-value WORD data, implied opcodes, indirect modes, labels with instructions, comments, `.ORG`, and repeated lines. In the current estimator operand checks run before directive checks, and accumulator operands fall through to a default size.
6. Route every emitted instruction through tracking. Direct `self.output.push` instructions, including startup loads, must not bypass byte accounting. Mark compiler-generated code separately instead of assigning it to the last user declaration.
7. Resolve line hover, breakpoint addresses, and active-line highlighting through the same lookup utility. Search within bank/file/range; do not assume globally increasing addresses across bank changes or `.ORG`.
8. Export an actual `sourcemap.json` with downloaded ROM builds as required by phase 35, as well as sending the map to Run. Verify both mapping directions.

**Done:** a line in an included file resolves to real instruction boundaries in the correct bank, and hovering, pausing, and the exported map agree.

### R05 — Build reusable runtime and browser acceptance harnesses

**Files:** `emulator/Cargo.toml`, `emulator/src/lib.rs`; **new** `emulator/src/core.rs`, `tests/support/`, `emulator/tests/`, browser tests.

1. Separate host-neutral emulator operations from wasm-bindgen exports without changing browser behavior. Produce an `rlib` as well as `cdylib` where needed, or add a host-neutral workspace crate if the separation merits it. Avoid a dependency cycle between server and emulator.
2. Expose load-bytes, reset, bounded execution, one-instruction execution, CPU snapshot, and non-mutating RAM/bus inspection to native tests. Keep browser `step` compatibility until its consumers migrate.
3. Configure deterministic RAM, region, and reset behavior for fixtures. Do not silently change normal application power-on behavior just to stabilize tests.
4. Execute compiled SwissBASIC through the production backend. Use a completion marker in explicitly fixture-owned memory or a resolved label. Avoid `$0300` (audio state) and `$07F0` (currently assumed bank state) as generic scratch locations.
5. Produce useful failure output: source fixture, ROM hash, current PC/bank, registers, expected/actual memory, and a bounded recent trace. Ensure tracing/debug peeks have no emulation side effects.
6. Create browser helpers for new project, file edit, save/reload, compile/run, pause/reset, and opening viewers. Wait for actual readiness events, not arbitrary sleep durations.
7. Keep visual/audio integration fixtures small and authored for this repository. Add input simulation for automated tests and a separate physical-gamepad acceptance run.

**Done:** future phase tasks can add behavioral tests without reimplementing compilation or creating a second emulator path.

## 5. Close phases 1–10: language and project foundations

For each phase below: inspect its listed tests, implement the steps, run its fixture through R05, then attach the evidence. Existing tests are starting points, not instructions to rewrite working modules.

### Phase 1 — Foundation and roadmap

**Start:** `DESIGN.md`, `AGENTS.md`, `README.md`, `.github/workflows/rust.yml`, `tests/foundation_test.rs`.

1. Complete R01 and reconcile phase numbering/status in all three documents.
2. Correct the startup URL to the actual `127.0.0.1:3000` binding and replace the placeholder clone URL.
3. Document source layout, tool versions, WASM generation, and how to reproduce tests; remove assumptions that tools installed inside one contributor's machine are available everywhere.
4. Confirm root and emulator packages compile on the CI host matrix and in WASM where appropriate.

**Accept:** fresh checkout instructions work, fmt/Clippy pass, and every status claim links to evidence.

### Phase 2 — String data

**Start:** lexer/parser/AST, symbol table, string helpers; `tests/string_test.rs`, `string_literal_test.rs`, `string_len_test.rs`, `string_functions_test.rs`, `heap_logic_test.rs`.

1. Compile `DIM name AS STRING = "Hello"` through the production pipeline.
2. Verify the emitted pointer, terminator/length representation, read-only literal placement, and initialization at runtime.
3. Test empty strings, two literals, assignment, concatenation, LEN, and the documented character encoding. Preserve working extended functions.
4. Test heap boundary slots, including slots above 15, exhaustion, and reuse behavior; assert no overwrite of adjacent memory.
5. Reject invalid type use with file/line diagnostics.

**Accept:** initialized string content and length are correct after execution, and heap limits fail safely.

### Phase 3 — DATA / READ / RESTORE

**Start:** `tests/data_read_test.rs`, `data_restore_test.rs`, `read_string_test.rs`.

1. Define/test byte and word encoding order and the width consumed by each READ.
2. Execute a program reading successive known values into RAM; include values above 255 for WORD.
3. Test RESTORE to start and to a named data block, then read the first expected item again.
4. Test data in included files and specify behavior on exhaustion/type mismatch. Verify generated data helpers exist in banked compilation.

**Accept:** the observed RAM sequence matches the data and restore positions; no read silently falls into unrelated ROM.

### Phase 4 — Multi-file projects

**Start:** preprocessor, project API, project frontend; `tests/project_files_test.rs`, include unit tests.

1. Create `main.swiss` and `lib.swiss` through the UI; include the latter and call its subroutine.
2. Save/reload both files, then compile through the named-project API, not a manually concatenated test source.
3. Verify the Compile action saves the edited secondary file but still compiles the project's entry point. Show the filename in compiler diagnostics.
4. Test missing include, direct/indirect include cycles, repeated include policy, case rules, and disallowed paths.
5. Test empty project/file names and reserved metadata names. Existing character validation permits empty strings; require meaningful names before performing filesystem operations.
6. Verify source-map identity and breakpoints in both files after R04.

**Accept:** Main invokes the included routine and produces its expected RAM result; all files survive reload without source confusion.

### Phase 5 — WORD addition and subtraction

**Start:** `tests/math_16bit_test.rs`, `poke_word_test.rs`.

1. Execute `1000 + 500` and assert little-endian result `$05DC` at the allocated variable address.
2. Test low-byte carry, high-byte carry, borrow, zero, `$FFFF`, mixed BYTE/WORD operands, and nested expressions.
3. Document unsigned wrap semantics and verify constants and variable expressions behave consistently.

**Accept:** observed values match the documented 16-bit model, not merely the expected ADC/SBC text.

### Phase 6 — Multiply, divide, and signed arithmetic

**Start:** `tests/math_advanced_test.rs`, math helpers in codegen.

1. Execute `200 * 50` and verify 10,000; include multiplication by zero and one and a wrapped large product.
2. Verify division quotient/remainder, divisor one, dividend smaller than divisor, and defined divide-by-zero behavior.
3. Test INT boundaries −128, −1, 0, 1, 127 and comparisons across the sign boundary; `-5 < 10` must be true.
4. Test mixed signed/unsigned promotion and arithmetic inside subroutine arguments. Protect intermediate values from helper clobbering.
5. Preserve supported MOD/ABS/SGN behavior and test edge cases where two's-complement limits matter.

**Accept:** runtime results and comparison branches match the chosen type semantics, including nested expressions.

### Phase 7 — SELECT CASE

**Start:** `tests/select_test.rs`, parser and codegen.

1. Execute several cases that write different markers; include default and unmatched selection.
2. Cover WORD values, ranges, comparisons, multiple case expressions if supported, and nesting inside IF/loops.
3. Verify the selected expression is evaluated according to documented semantics and there is no unintended fallthrough.
4. Confirm stack balance through all exit paths and a clear diagnostic for invalid CASE syntax.

**Accept:** the fixture behaves identically to the equivalent IF/ELSE chain.

### Phase 8 — Structures

**Start:** `tests/struct_test.rs`, `array_struct_test.rs`, `array_test.rs`.

1. Define a player record with BYTE, WORD, and nested fields; verify layout and total size.
2. Execute field writes/reads for two instances and an array of records, checking neighboring fields remain unchanged.
3. Test a WORD field crossing a page boundary, missing fields, duplicate fields, and recursive/invalid type declarations.
4. Recheck R03 limits using a large structure/array allocation and parameter storage.

**Accept:** field values, offsets, and allocation limits are correct in RAM.

### Phase 9 — Enums and constants

**Start:** `tests/enum_test.rs`, semantic analysis.

1. Use the parser's actual ENUM grammar; replace any illustrative unsupported brace syntax in documentation.
2. Test implicit incrementing, explicit values, negative values where supported, member qualification, and name collisions.
3. Use enum members in assignments and SELECT CASE and verify the resulting behavior.
4. Reject out-of-range assignments rather than silently truncating without a documented rule.

**Accept:** enum and equivalent constant versions of a fixture produce the same result.

### Phase 10 — Macros

**Start:** `src/compiler/preprocessor.rs`, `tests/macro_test.rs`.

1. Expand a parameterized macro used twice with different inputs and execute both expansions.
2. Cover expression arguments, nested macros, wrong arity, and direct/indirect recursion limits.
3. Verify local/generated labels cannot collide across expansions; preserve source call sites for diagnostics and debugging.
4. Run INCLUDE plus macro expansion together through `compile_source`.

**Accept:** expanded behavior is correct and invalid/recursive expansion terminates with an actionable error.

## 6. Close phases 11–20: game runtime

### Phase 11 — Controller library

**Start:** `tests/controller_test.rs`, controller helpers, `editor.js` input.

1. Verify controller strobe/read order using a scripted eight-button state.
2. Test unpressed → pressed → held → released across consecutive game frames; assert IsPressed/IsHeld/IsReleased independently.
3. Validate button constants and the project's true value `$FF` consistently.
4. Run the fixture through the banked API path and later through keyboard/gamepad acceptance in phase 33.

**Accept:** edge and held state outputs match the input script and do not clear one another unexpectedly.

### Phase 12 — Text engine

**Start:** `tests/text_test.rs`, text helpers, CHR/font fixtures.

1. Create an authored ASCII-to-tile font fixture with known CHR bytes.
2. Print “Hello World” at `(10,10)` and inspect both nametable bytes and rendered output.
3. Verify custom character offset/mapping, empty strings, and the documented behavior at screen edges.
4. Respect the existing direct-PPU-write contract: use WAIT_VBLANK and keep writes within budget, or route them through the existing buffered approach with tests. Avoid promising arbitrary-length safe printing without a bound.

**Accept:** the expected tiles appear at the specified coordinates without rendering corruption.

### Phase 13 — Metasprites

**Start:** `tests/metasprite_test.rs`, sprite helper generation.

1. Draw an authored 16×32 character of eight tiles with distinct tile IDs and offsets.
2. Inspect all eight shadow OAM entries and the displayed character; verify signed relative offsets and attributes.
3. Test the 64-sprite boundary, offscreen coordinates, and transparent color behavior.
4. Enable flicker/cycling with more than eight sprites on a line. Verify cycling changes which sprites survive; it cannot remove the hardware scanline limit.

**Accept:** composition, attributes, DMA output, and documented overflow behavior are correct.

### Phase 14 — Animation

**Start:** `tests/animation_test.rs`, animation helpers.

1. Define three distinguishable frames with explicit durations; step game frames deterministically.
2. Assert frame changes exactly at duration boundaries, loops correctly, and non-looping animation holds/stops as documented.
3. Test restart, switching animations, missing metasprites, and zero/invalid duration handling.
4. Verify sprite-editor export later uses the same frame format.

**Accept:** a moving character cycles three frames with correct timing, and stopping/resetting behaves predictably.

### Phase 15 — Object pools

**Start:** `tests/pool_test.rs`, `Pool.Spawn`/`Pool.Despawn` codegen.

1. Allocate capacity ten; spawn ten entities and verify each slot is unique and initialized.
2. Attempt an eleventh spawn and verify the documented failure sentinel with no overwrite.
3. Despawn a middle slot and verify the next spawn reuses available storage.
4. Exercise complex expressions for base/index arguments, invalid indices, and double-despawn; assert stack and neighboring RAM remain intact.

**Accept:** capacity and reuse behavior are deterministic and safe.

### Phase 16 — AABB collision

**Start:** `tests/collision_test.rs`, collision helpers.

1. Define overlap semantics explicitly, including whether touching edges count.
2. Execute cases for separated, overlapping, contained, touching, zero-width/height, and reversed argument order.
3. Test positions near 255 and supported 16-bit positions; do not allow arithmetic overflow to produce a false hit.

**Accept:** all rectangle fixtures return the expected true/false result in RAM.

### Phase 17 — Point and tile collision

**Start:** collision tests, nametable data and tile lookup helpers.

1. Make a map with one solid tile and known empty neighbors.
2. Test pixel-to-tile lookup around 7/8-pixel boundaries, map edges, and out-of-range coordinates.
3. Specify whether coordinates are screen or world coordinates and how scroll offsets enter the conversion.
4. Run a character into the solid tile and verify movement stops; include a second screen once phase 29 is closed.

**Accept:** collision refers to the tile actually displayed at the tested location.

### Phase 18 — Horizontal scrolling

**Start:** `tests/scroll_test.rs`, `scroll_column_test.rs`, `scroll_gap_test.rs`.

1. Create distinguishable adjacent screens and scroll through repeated nametable boundaries.
2. Verify fine/coarse scroll, nametable select, mirroring, and offscreen column data and attribute writes.
3. Test VBlank buffer capacity and overflow behavior; split or defer uploads so code does not write outside the buffer.
4. Record at least two full boundary crossings with no seams or stale attributes; also inspect buffered bytes in automated tests.

**Accept:** repeated horizontal scrolling preserves graphics and palette attributes without visible seams.

### Phase 19 — Vertical scrolling

**Start:** `tests/scroll_row_test.rs`, `scroll_gap_test.rs`, scroll helpers.

1. Repeat phase 18 checks vertically, accounting for 30 visible tile rows and vertical nametable transitions.
2. Test upward and downward movement, fine-Y boundaries, attributes, and mirroring configuration.
3. If the project implements a status bar, verify the existing split handling; otherwise document that conditional roadmap requirement as not applicable and do not add a new IRQ renderer.
4. Test changing direction near a seam without corrupting row uploads.

**Accept:** up/down scrolling is visually correct and upload buffers remain bounded.

### Phase 20 — Random number generator

**Start:** `tests/rng_test.rs`, RNG helpers.

1. Expose/use a deterministic seed for testing and verify the same seed produces the same sequence.
2. Test zero seed handling and avoid a locked all-zero LFSR state.
3. Run RND(100) repeatedly and assert each value lies in 0–99 and the sequence varies.
4. Define RND(0), RND(1), and supported upper bounds; verify the resulting behavior without claiming cryptographic or perfectly uniform randomness.

**Accept:** range and repeatability requirements pass, and normal seeding produces changing gameplay values.

## 7. Close phases 21–30: audio and visual authoring

### Phase 21 — DPCM samples

**Start:** `src/compiler/audio.rs`, `static/js/audio.js`, `tests/audio_dpcm_test.rs`.

1. Import a short, authored WAV with known sample rate/channel count. Verify decoding, mono conversion, resampling policy, and 1-bit delta encoding.
2. Document one channel-ID table shared by frontend, persisted assets, compiler, and runtime. Current code/comments disagree about channel 3 being Noise or DMC; use behavior tests to resolve the actual mapping and migrate saved assets if needed.
3. Verify 64-byte sample start alignment and NES DMC length encoding (`16*n+1` bytes), including padding, minimum/maximum lengths, and sample table indices.
4. Check samples fit their assigned ROM region and remain accessible under the compiler's bank configuration. Do not overwrite vectors or wrap the sample region silently.
5. Trigger the sample using controller input. Capture decoded sample bytes/register activity and listen through the browser emulator.
6. Test malformed/unsupported WAVs and overlarge samples with useful errors, preserving the project on rejection.

**Accept:** an imported sample plays on button press with correct duration and survives save/reload/compile.

### Phase 22 — SFX priority

**Start:** `tests/audio_priority_test.rs`, sound engine and SFX data.

1. Define the priority comparison and equal-priority policy; the roadmap requires higher priority to interrupt the current music note.
2. Start music, trigger a lower-priority SFX, then a higher-priority SFX on the same channel.
3. Verify lower priority does not replace music, higher priority does, and completion returns the channel to the intended music state.
4. Test equal priority, repeated triggers, different channels, and looping effects; inspect engine state/register writes as well as listening.

**Accept:** the three priority cases behave consistently and music resumes without stale envelope state.

### Phase 23 — Envelope UI

**Start:** `static/js/envelopes.js`, music editor, `AudioEnvelope`, `compile_envelopes`.

1. Draw a volume fade and a signed pitch curve with distinct steps/durations.
2. Save/reload, switch tracks, and verify the exact arrays and loop position survive.
3. Compile and compare encoded envelope data to the authored curve; inspect playback at each step boundary.
4. Test empty/max-length envelopes, duration limits, invalid loop indices, and imported invalid values at both UI and API boundaries.

**Accept:** playback follows the drawn envelope and invalid data cannot overflow envelope ROM storage.

### Phase 24 — Arpeggios

**Start:** music `arpeggio_env`, envelopes and runtime pitch tables.

1. Create a 0,4,7 semitone sequence, attach it to a sustained note, and inspect successive generated periods.
2. Verify looping, speed/duration, negative offsets, note-range bounds, and base-note preservation.
3. Test simultaneous volume and pitch envelopes and save/reload of all references.
4. Listen to the authored chord effect; retain numeric period assertions so playback is not judged only by ear.

**Accept:** one channel follows the intended arpeggio without accumulating unintended pitch drift.

### Phase 25a — SFX engine core

**Start:** `SoundEffect` in `src/server/project.rs`, `compile_sfx_data`, runtime SFX helpers, `tests/sfx_compilation_test.rs`.

1. Document the binary layout: table entry, channel, priority, speed, loop semantics, sequence lengths, signed pitch encoding, and terminators.
2. Compile an explicit effect and assert its exact bytes, table address, and boundaries.
3. Execute PLAY_SFX and verify volume/pitch/duty progression, speed handling, stop, and looping.
4. Exercise priority from phase 22, missing IDs, empty sequences, and different sequence lengths.

**Accept:** manually authored SFX data executes correctly through the full compile/ROM path.

### Phase 25b — SFX UI foundation

**Start:** `static/js/sfx.js`, `static/index.html`, project asset plumbing.

1. Verify Music and SFX are clearly labeled and their data is stored independently.
2. Create, rename, duplicate if supported, select, and delete an effect. Set channel, priority, speed, and loop properties.
3. Save/reload the project and compare the effect list and properties, including an empty list.
4. Handle deletion of a referenced effect deterministically: repair references or report a validation error, without silently playing another effect.

**Accept:** named effects and properties persist correctly and the UI has no stale selection state.

### Phase 25c — Visual SFX envelopes

**Start:** SFX canvas controls and `SoundEffect` schema.

1. Draw volume fade, pitch slide, and duty steps; verify signed pitch and legal channel-specific duty/volume ranges.
2. Test drag endpoints, resizing a sequence, empty data, and loop boundaries. Confirm sequence edits actually mark the project dirty.
3. Close the roadmap's SFX arpeggio requirement. The baseline `SoundEffect` has volume/pitch/duty sequences but no explicit arpeggio sequence. Determine whether pitch sequence currently represents semitones or another unit; document that evidence.
4. If absent, add an optional arpeggio field with serde defaults, UI editing, compiler encoding, and runtime interpretation together. Define how it combines with pitch; test old JSON loading without the field.
5. Execute a visually designed effect and compare its generated sequence with the authored data.

**Accept:** volume, pitch, duty, and required arpeggio editing work end to end, including older saved projects.

### Phase 25d — SFX import/export and integration

**Start:** SFX drag/drop/import/export handlers, compiler API.

1. Export an effect as `.sfx.json`, import it into a different project, and compare all fields including newly added fields.
2. Drop that file onto the intended UI target and verify the correct effect becomes selected.
3. Validate shape, types, ranges, channel, and sequence lengths before replacing state. Reject malformed JSON without partially mutating the project.
4. Test cancellation, duplicate names, and importing a file with omitted optional fields.
5. Compile and trigger the imported effect in-game; compare with the original effect's output.

**Accept:** the exact export → drop → save → compile → audible playback path works.

### Phase 26 — CHR import

**Start:** `static/js/chr.js`, palette editor, CHR data injection.

1. Prepare a 128×128 PNG containing identifiable tile corners and all four chosen palette colors.
2. Import by the documented file/drop path and verify conversion to 256 NES 8×8 tiles in the expected planar bit format.
3. Verify transparency maps to color 0 and nearest-color conversion uses the selected four-color palette. Define tie handling deterministically.
4. Test invalid dimensions, non-PNG data, cancellation, and reimport without corrupting other asset state.
5. Save/reload, compile, and compare decoded CHR bytes and rendered tiles. Explain that a 4 KiB authoring bank occupies part of the current 8 KiB output.

**Accept:** imported tile identities/colors match both the editor and the produced ROM.

### Phase 27 — Metatile editor

**Start:** `static/js/metatile.js`, `Metatile` schema, metatile persistence tests.

1. Create a 16×16 block from four distinct tiles, assign palette 0–3, name it, and save/reload.
2. Verify quadrant order matches Rust `[u8;4]` and map painting orientation.
3. Test tile replacement, palette change, deletion, duplicate names, and reference handling.
4. The roadmap permits 16×16 **or** 32×32 blocks; baseline supports four-tile 16×16 blocks. Keep that valid scope unless 32×32 is separately requested.

**Accept:** a reusable 16×16 block paints the expected four tiles and palette.

### Phase 28 — Metatile integration

**Start:** `static/js/map.js`, `metatile_grid`, attribute persistence, compiler metatile data.

1. Paint metatiles in a map, then edit an individual tile in that area.
2. Specify whether this detaches that map cell from its metatile; keep flattened tiles and `metatile_grid` consistent.
3. Paint across attribute quadrant boundaries and verify two-bit palette fields preserve neighboring cells.
4. Change/delete a metatile and verify referenced map/world/sprite data updates or reports a clear missing reference according to one documented policy.
5. Save/reload, compile, and inspect displayed tiles and attributes.

**Accept:** individual-tile and metatile workflows coexist without divergent persisted/rendered data.

### Phase 29 — World editor and ROM integration

**Start:** `static/js/world.js`, `WorldLayout`, asset injection in `src/server/api.rs`, `tests/world_compilation_test.rs`.

1. Create four distinguishable maps and arrange them in a 2×2 world with an additional empty cell case.
2. Verify adjacency, dimension changes, map replacement, deletion, and `-1` empty-cell handling. Use checked `width*height` validation and require matching data length.
3. Close the production gap: the API currently injects only the first nametable. Define storage for every world-referenced map, with a directory of bank/offset/length references, or another bounded format compatible with the compiler's layout.
4. Emit that directory and map data through the same linker/segment allocation machinery from R03. A list of map indices alone is insufficient; the referenced map bytes must exist in ROM.
5. Connect the runtime's map loading/streaming operations to these references. Preserve mirroring and scroll buffer limits. Report a clear size limit when available ROM space is exhausted.
6. Test transitions to all four maps, backtracking, an empty cell, and map attribute preservation.
7. Confirm existing one-map projects migrate without data loss.

**Accept:** every placed map survives authoring, persistence, compile, and runtime navigation; the world UI is not just a saved layout picture.

### Phase 30 — Sprite and animation editor

**Start:** `static/js/sprite.js`, `Metasprite`, `Animation`, injection in API.

1. Create an original multi-tile character with offsets and palette/flip attributes; exercise metatile placement if the UI advertises it.
2. Build a three-frame walk with different durations, reorder frames, and set loop mode.
3. Save/reload, compile exported declarations/assets, and invoke them by the documented language names.
4. Validate identifier collisions, deleted frame references, offsets outside the signed-byte range, empty animations, and sprite count limits.
5. Compare editor preview, emitted data, OAM, and runtime animation.

**Accept:** a character created wholly in the UI can be animated in a compiled program with the intended timing.

## 8. Close phases 31–38: browser emulator and debugging

### Phase 31 — WASM integration

**Start:** `emulator/src/lib.rs`, `static/wasm/`, editor dynamic import, compile/run events.

1. Complete the reproducible WASM build from R01 and test a fresh build rather than relying on tracked binaries.
2. Verify module initialization returns the correct memory object, exported methods exist, and ROM-load errors become visible UI errors.
3. Define each buffer's format/dimensions/length/lifetime. TetaNES `frame_buffer()` is the filtered byte buffer; `frame_buffer_raw()` is a different `u16` buffer. Do not guess based on length alone.
4. Copy/read typed-array data before a call that could grow WASM memory or reallocate the backing vector. Recreate views from the current `memory.buffer` after such calls.
5. Run an authored palette test and moving object. Verify channels, transparency, full 256×240 frame, and absence of out-of-bounds reads.
6. Load a second ROM, an invalid ROM, then a valid ROM again. Verify one emulator instance/loop owns the active session and errors do not require reloading the page.

**Accept:** clicking Run builds and plays the current project from a fresh checkout with known-correct pixels.

### Phase 32 — Emulator controls and scheduling

**Start:** editor overlay, `emulatorLoop`, audio scheduling.

1. Verify play/pause freezes/resumes CPU state, reset restarts execution, and 1×/2×/fullscreen scaling preserves aspect ratio and nearest-neighbor pixels.
2. Decouple emulated frame advancement from browser refresh rate. The current loop advances one NES frame per requestAnimationFrame; test 60 Hz, high refresh, and background-tab recovery.
3. Use elapsed time with a capped catch-up budget or an equivalent documented pacing scheme. Reset accumulated time on resume so a hidden tab does not run thousands of frames at once.
4. Test pause → resume and repeated Run clicks for duplicate animation loops and keyboard handlers.
5. Use consistent gain control; verify slider changes affect current playback and queued buffers do not continue after stop/reset. Rebase audio scheduling after pause/reload.
6. Close the overlay and verify CPU/audio/input resources stop; reopen and verify one clean session.

**Accept:** speed and controls remain correct across refresh rates, repeated sessions, and tab visibility changes.

### Phase 33 — Keyboard and gamepad

**Start:** editor input state and Gamepad API polling.

1. Verify Z/X, Shift, Enter, and arrows map to the intended NES buttons and only capture keys while emulator input is active.
2. Verify a controller using the browser's standard mapping, including face buttons, Start/Select, D-pad, axes, and a documented dead zone.
3. Hold a button on keyboard and gamepad, release one, and verify the other remains pressed. Test all release combinations.
4. Clear physical and last-sent state on blur, disconnect, stop, load, and reset as appropriate; re-send currently held input when necessary.
5. Handle missing Gamepad API, no connected pads, device indices changing, and unplug/replug without a crash or stuck input.
6. Run at least one physical Xbox-style controller test in addition to mocked automated input.

**Accept:** the phase 11 player moves correctly, including mixed-device holds and disconnections.

### Phase 34 — Debug protocol

**Start:** CpuState exports, RAM getters, R05 core interface.

1. Document CPU snapshot fields, widths, units, and whether the snapshot is before or after instruction execution. Use a stable representation for cycle counts across 32-bit WASM and native hosts.
2. Return coherent CPU/memory observations while paused or at a defined execution boundary. Avoid exposing references that are invalidated by the next call without documenting that lifetime.
3. Provide side-effect-free peeks for debugger use; tests must prove inspecting PPU status/controller ports does not acknowledge/shift emulated state.
4. Query the mapper/bus for address identity. Remove `$07F0` as the general debugger's source of physical bank identity; it is compiler-specific and incorrect for arbitrary ROMs and fixed-bank addresses.
5. Free wasm-bindgen-owned snapshot objects or use a documented value serialization strategy; test repeated polling for bounded memory use.

**Accept:** JS can reliably read registers, cycles, and the full 2 KiB CPU RAM before/after a known instruction.

### Phase 35 — Source maps

**Start:** complete R04.

1. Compile a multi-file, multi-bank fixture and independently compare mapped starts/ends against actual assembled instructions.
2. Hover executable lines, non-executable declarations, included-file lines, and macro call sites; show accurate addresses or an explicit “no executable address.”
3. Verify two mappings with the same CPU address but different banks remain distinct.
4. Download `sourcemap.json` and verify it corresponds to the downloaded ROM hash/revision.

**Accept:** both source→address and address→source work, including file/bank identity and generated-code exclusions.

### Phase 36 — Breakpoints

**Start:** `Emulator::step`, breakpoint exports, editor gutter.

1. Define the pause contract as **before executing the instruction at the resolved breakpoint**. The current unconditional first instruction in `step()` needs a test for a breakpoint at the initial PC.
2. Add resume-once semantics: after pausing, continuing executes that instruction once, then re-arms the breakpoint. A loop returning to it must pause again.
3. Test initial-PC breakpoint, two breakpoints, removal, clear-all, same-address different-bank code, fixed-bank code while a different switchable bank is active, and included-file lines.
4. Separate user pause, breakpoint, frame completion, and fault results, internally if preserving the old boolean export temporarily. Apply an instruction budget to avoid an unbounded browser call.
5. On hit, update registers/RAM/source highlight and render the current frame before stopping. Avoid displaying a stale pre-hit frame.
6. Invalidate/re-resolve breakpoints on compile/reload and prevent accidental execution of stale ROM/map combinations.

**Accept:** the game stops at the intended source instruction without executing its side effects, and continue/re-hit behavior is correct.

### Phase 37 — Memory viewer

**Start:** editor RAM viewer, get_wram exports.

1. Verify display covers `$0000–$07FF`, with correct hex addresses, byte formatting, and boundaries between zero page, stack, OAM, audio, buffers, strings, and user variables.
2. Meet the roadmap's virtualized-list requirement or implement an equivalent visible-row rendering approach with measured bounded update work. Define whether this phase is read-only; RAM patching is not required until the later debugger expansion.
3. Update on breakpoint/pause immediately and at a bounded cadence while running. Avoid rebuilding hidden views each frame.
4. Execute a fixture changing a known user variable and verify displayed bytes. `$0300` is sound-engine state, so use it only when intentionally inspecting audio.
5. Test empty/unloaded emulator, reset/reload, WASM memory growth, and viewport scrolling without stale typed arrays.

**Accept:** changing bytes and paused values are accurate across the complete internal RAM range without destabilizing emulation.

### Phase 38 — PPU viewer

**Start:** `ppu_viewer.js`, emulator buffer exporters, dependency PPU debug functions.

1. Define exact RGBA dimensions: patterns 256×128; nametables 512×480; palette output from the inspected core 16×2 (32 pixels / 128 bytes). Export actual lengths; render swatches with CSS/canvas scaling instead of treating padding as colors.
2. Keep the viewer hidden at construction. The current constructor sets `display='none'` then overwrites it with `display='flex'`.
3. Verify both pattern tables, all four nametable quadrants, palette mirroring, attributes, and OAM fields against a diagnostic ROM. Use current mapped CHR data after bank changes.
4. Add the current viewport/scroll indication required to understand the scrolled view. Decode sprite attributes and Y-coordinate semantics, and make offscreen sprites inspectable.
5. Test live updates, paused updates, tab changes, close/reopen, ROM unload/reload, and a missing memory/emulator handle. Avoid duplicate update loops.
6. Replace guessed dimensions/comments with a buffer contract verified by tests. Bound every array access before creating ImageData.

**Accept:** the developer can locate an offscreen sprite and inspect the actual CHR/nametable/palette state used by the running game.

## 9. Phase 39: repository ingestion and bus/cartridge architecture

**Required exit:** supported iNES/NES 2.0 headers are parsed correctly, unsupported cartridge hardware produces a structured error, the production CPU uses a tested 16-bit bus, and the named mapper abstraction is wired into that execution path.

### P39-01 — Record implementation ownership and close architecture gaps

**Prerequisite:** M5. **Start:** emulator wrapper and TetaNES `cart.rs`, `bus.rs`, `mapper.rs`, `control_deck.rs`.

1. Write the architecture record started in section 1. Diagram the actual ownership chain: browser → WASM adapter → ControlDeck/core → CPU bus → PPU/APU/input/cartridge mapper. Include native test/CLI entry points using the same core.
2. Inventory existing interfaces and coverage against each P39 requirement. Record exact dependency version/checksum or patched revision. Identify behavior that can be tested publicly versus changes requiring dependency integration.
3. Retain the existing core by default. Introduce host-neutral cartridge metadata, debug bus access, and bounded execution in project-owned modules as needed.
4. For dependency gaps, first add a failing integration fixture. Apply a small pinned patch/fork using Cargo's supported patch mechanism, or adopt a compatible upstream version only after full regression comparison. Never edit a Cargo registry cache as the implementation.
5. Keep one source of truth for cartridge sizes/mirroring and bus state. A preflight parser must feed/agree with the actual loaded cartridge; parsing correctly and then handing the original bytes to a loader that rejects/misreads them does not complete phase 39.
6. Record borrowed-code license notices and the update procedure for any maintained patch. Do not vendor a complete new emulator merely to rename its interfaces.

**Accept:** a reviewer can follow the actual production path and identify where every requirement lives and is tested.

### P39-02 — Define normalized cartridge metadata and errors

**New/proposed:** `emulator/src/cartridge.rs` or the equivalent module in the extracted core; `emulator/tests/header_test.rs`.

1. Define metadata with format, mapper ID (`u16`, preserving 12 bits), submapper, mirroring mode, trainer flag/bytes, battery flag, PRG/CHR ROM byte sizes, volatile/nonvolatile PRG/CHR RAM byte sizes, and timing/console information.
2. Separate parse errors from unsupported hardware. A valid NES 2.0 header for an unsupported mapper should be recognized and reported as unsupported, not called corrupt.
3. Define explicit errors for short header, bad magic, impossible/truncated payload, arithmetic overflow, policy size limit, unsupported format/console, and unsupported mapper/submapper.
4. Parse bytes/slices in the shared core; filesystem/UI code only supplies bytes. Do not require paths or host filesystem access in WASM.
5. State practical allocation limits and validate claimed sizes against available payload before allocating. Preserve useful offset/expected/actual information in errors.

**Accept:** native and WASM callers receive the same normalized metadata or equivalent structured failure.

### P39-03 — Implement and test iNES decoding

1. Check the 16-byte header and `NES` plus `$1A` magic before accessing fields.
2. Decode legacy PRG/CHR bank counts into byte sizes, mapper nibbles, mirroring, battery, and trainer flag. Distinguish legacy format policy from NES 2.0 detection.
3. Handle the optional 512-byte trainer when slicing PRG/CHR payload. Map its runtime semantics consistently with the supported board, or report that load capability explicitly; full trainer handling is the target in this plan because the baseline rejects all trained images.
4. Define legacy zero-CHR-ROM behavior (CHR RAM) and PRG RAM defaults with reference to board/format conventions, not a blanket rule copied to NES 2.0.
5. Define treatment of archaic/dirty headers and trailing bytes. Do not silently infer unknown hardware from a corrupt mapper field without documenting the compatibility rule.
6. Validate header+trainer+PRG+CHR sizes with checked sums and exact short-read diagnostics.

**Fixtures:** NROM 16 KiB PRG, NROM 32 KiB PRG, CHR ROM, CHR RAM, horizontal/vertical/four-screen flags, mapper nibble combination, battery RAM, trainer with distinctive prefix, header lengths 0–15, bad magic, truncated trainer/PRG/CHR, and trailing bytes policy.

**Accept:** metadata and byte slicing match fixture expectations, including trainer offsets; valid supported images load through the production backend.

### P39-04 — Complete NES 2.0 sizes and metadata

1. Detect NES 2.0 using the specified header format bits, preserving extended mapper and submapper fields.
2. Decode linear PRG/CHR sizes and exponent/multiplier size encoding separately. In exponent mode the size is bytes, not an additional bank count. Use checked arithmetic and allocation limits for large exponents.
3. Decode volatile and nonvolatile RAM nibbles independently. A zero RAM shift denotes absence; nonzero values use the format's shift-size encoding. Do not substitute legacy defaults when NES 2.0 explicitly declares absence.
4. Preserve timing/console/miscellaneous metadata sufficiently to reject unsupported variants accurately rather than silently emulating them as ordinary NTSC cartridges.
5. Resolve the TetaNES 0.12.2 parser limitations with actual load-path patches/adaptation and integration tests. The inspected code alone does not establish complete format support.
6. Add small synthetic fixtures that exercise each encoding without allocating enormous buffers. Test overflow declarations as failures before allocation.

**Fixtures:** mapper above 255, nonzero submapper, PRG and CHR linear high bits, PRG exponent mode, CHR exponent mode, zero/nonzero RAM/NVRAM fields, CHR-RAM-only image, mixed RAM declarations, unsupported console/timing, overflow, and truncated payload.

**Accept:** each metadata field has independent expected-value tests and the supported formats also execute through the loader. The relevant size encodings are specified in [NES 2.0](https://www.nesdev.org/wiki/NES_2.0); keep those rules beside the parser tests.

### P39-05 — Define and connect the Mapper contract

**New/proposed:** mapper adapter/trait in project-owned core or a narrow maintained dependency patch; mapper contract tests.

1. Define the roadmap operations `read_prg`, `write_prg`, `read_chr`, `write_chr`, and `step_irq`. Document CPU versus PPU address domains and ownership of ROM/RAM storage.
2. Add side-effect-free peek/mapping lookup and mirroring queries as needed by phases 34–38. Return a clear unmapped/open-bus indication instead of inventing data for unhandled addresses.
3. Specify IRQ inputs and scheduling. A parameterless “once per instruction” tick cannot represent every future mapper; allow the required CPU-cycle or PPU-bus event context while retaining the requested conceptual operation.
4. TetaNES currently uses `MapRead`, `MapWrite`, `MappedRead`/`MappedWrite`, bus event hooks, and an enum-backed mapper system. Implement a bridge at that dispatch/storage boundary so the named operations are actually invoked by CPU/PPU execution. If its public API cannot support the bridge, include that narrow change in the pinned core patch; do not bolt on an unused parallel bus.
5. Add a recording mapper used in an integrated test to prove CPU accesses, PPU CHR accesses, and scheduled IRQ events reach the contract with the right addresses/values. Verify a mapper IRQ reaches the CPU line; do not implement a new MMC3 solely for this test.
6. Preserve existing mapper-1 compatibility and provide NROM for nestest. Phase 42's broader mapper deliverables remain out of scope.

**Accept:** the actual emulator path uses the abstraction; production and recording-mapper tests agree on routing, reads, writes, mirroring, and IRQ signaling.

### P39-06 — Audit/repair the full 16-bit CPU bus

**Start:** dependency bus and project core adapter; **new** `emulator/tests/bus_test.rs`.

| CPU range | Required route | Required test |
|---|---|---|
| `$0000–$07FF` | Internal 2 KiB RAM | Write/read first, middle, last bytes |
| `$0800–$1FFF` | Mirrors via `addr & $07FF` | All three mirrors alias the same underlying storage |
| `$2000–$2007` | PPU registers | Read/write direction and register side effects |
| `$2008–$3FFF` | PPU mirror every 8 bytes | Equivalent normalized register accesses at edges |
| `$4000–$4013` | APU registers | Correct routing without treating it as ordinary RAM |
| `$4014` | OAM DMA register | Scheduling hook and source page passed through |
| `$4015` | APU status/control | Distinct read/write behavior |
| `$4016` | Controller read / controller strobe write | Serial shift and latch behavior |
| `$4017` | Second controller read / APU frame-counter write | Read and write go to different owners |
| `$4018–$401F` | Disabled/test region under declared policy | Deterministic unmapped behavior, no accidental RAM |
| `$4020–$FFFF` | Cartridge/mapper space | Expansion/RAM/ROM handled according to mapper |

1. Trace representative accesses through the actual bus; normalize mirrors once and preserve peripheral side effects.
2. Test every range's start/end and the adjacent address. Add a property-style test across all internal RAM mirrors.
3. Track open-bus behavior consistently for unmapped/read-only locations. Keep debugger peeks non-mutating even where real reads have side effects.
4. Verify little-endian vector reads at `$FFFA–$FFFF` use mapped cartridge bytes, and PRG ROM writes invoke mapper logic instead of mutating immutable ROM storage.
5. Test two sequential ROM loads and two core instances for leaked RAM/mapper/interrupt state. TetaNES has static/thread-local interrupt helpers; verify reset and test isolation explicitly.
6. Keep existing PPU/APU devices connected. A bus skeleton used only in tests while production bypasses it is insufficient. Detailed new PPU/APU fidelity belongs to later phases.

**Accept:** the route table passes through the production bus. See [CPU memory map](https://www.nesdev.org/wiki/CPU_memory_map) for routing references.

### P39-07 — NROM fixture support, load lifecycle, and phase gate

1. Verify mapper 0 with 16 KiB PRG mirrored into both CPU windows and 32 KiB PRG mapped without that repetition.
2. Verify CHR ROM stays read-only and CHR RAM accepts writes; preserve mirroring metadata for the PPU.
3. Verify valid compiled mapper-1 games still load/reset/play; test a bank-switching fixture so preservation is not inferred from the header alone.
4. Load malformed data after a valid game and define transactional behavior: preserve the old session or reset to a safe unloaded session with an explicit error. Test that policy.
5. Verify debugger address identity follows the currently mapped cartridge, including fixed windows, and never depends on arbitrary guest RAM.
6. Attach header-case counts, bus test results, mapper bridge evidence, native/WASM results, and compatibility fixture hashes to the phase 39 sign-off.

**Phase 39 done:** all P39 tasks pass. Known unsupported boards are explicit; no claim is made to complete phase 42.

## 10. Phase 40: Ricoh 2A03 CPU hardening

**Required exit:** the CPU used by the browser and native harness supports the official instruction set, required unofficial opcodes, and interrupt behavior, and compares without discrepancies against a pinned complete canonical nestest log. A nestest pass alone does not prove every timing interaction on NES hardware; supplemental tests below close the stated CPU requirements.

### P40-01 — Establish a CPU execution and trace contract

**Prerequisite:** P39-07. **Start:** R05 core, TetaNES `cpu.rs` and instruction modules. **New/proposed:** `emulator/src/trace.rs`, `emulator/src/bin/nes_trace.rs`.

1. Keep the CPU implementation from the selected/patched TetaNES version. Inventory its instruction table and semantics before writing replacements. Add only missing behavior or fixes proved by tests.
2. Define `step_instruction` to execute one instruction or a precisely reported interrupt/DMA event according to the selected core's boundary model. Record elapsed CPU cycles and resulting stop reason. Keep frame-step behavior distinct.
3. Define a trace record with pre-instruction PC, opcode bytes, A/X/Y/P/SP, total CPU cycles, and optional bus activity/PPU position. Use fixed-width integers internally; avoid platform-sized counters in the persistent trace format.
4. Keep tracing disabled in normal play; enable it deterministically in tests/CLI. CPU-state capture and operand inspection must use non-mutating peeks.
5. Set an instruction/cycle budget and surface a jam/fault as a result. A malformed ROM or KIL opcode must not trap the browser in an unbounded loop.
6. Test that enabling tracing does not change final CPU/RAM state or timing for the same fixture.

**Accept:** a native command executes the same backend as the browser and emits reproducible pre-instruction records.

### P40-02 — Audit all official opcodes and addressing forms

1. Create **new** `docs/architecture/cpu-opcode-coverage.md` with one row for every opcode `$00–$FF`: classification, mnemonic, addressing mode, bytes, base cycles, conditional cycles, flag changes, implementation symbol, and test IDs.
2. Verify coverage of **151 official opcode encodings / 56 mnemonic instructions**; do not mistake 56 mnemonics for 56 byte values or require every mnemonic in every mode. See the [instruction/opcode distinction](https://www.nesdev.org/wiki/Programming_Basics).
3. Reconcile the roadmap's “12 addressing modes”: explicitly test implied, accumulator, immediate, zero page, zero page X, zero page Y, absolute, absolute X, absolute Y, relative, indirect JMP, indexed-indirect X, and indirect-indexed Y. This is 13 forms when accumulator is counted separately. Document the counting convention instead of dropping accumulator coverage.
4. Verify instruction length, PC movement, effective address, read/write behavior, flags, and cycles for each official encoding. Unit tests should test behavior, not merely table presence.
5. Keep NES 2A03 semantics: decimal flag instructions/status still exist, but ADC/SBC remain binary. Do not import 65C02 behavior such as its fixed indirect-JMP behavior or extra instructions.

**Implementation/test batches:**

| Batch | Mnemonics | Required evidence |
|---|---|---|
| Loads/stores | LDA LDX LDY STA STX STY | Values, addressing, flags for loads, preserved flags for stores |
| Transfers | TAX TAY TXA TYA TSX TXS | Target registers and differing flag effects |
| Stack | PHA PHP PLA PLP | Stack addresses/order, wrap, pulled flag behavior |
| Arithmetic | ADC SBC | Carry-in/out, overflow, zero/negative, binary behavior with D set |
| Logic/compare | AND ORA EOR BIT CMP CPX CPY | Result and independent flag expectations |
| Inc/dec | INC INX INY DEC DEX DEY | Wrap boundaries, preserved flags, RMW writes |
| Shifts/rotates | ASL LSR ROL ROR | Accumulator/memory forms, carry-in/out, bus effects |
| Branches | BCC BCS BEQ BNE BMI BPL BVC BVS | Taken/not taken, positive/negative displacement, page timing |
| Flow/interrupt return | JMP JSR RTS BRK RTI | Destination, stack contents, return address, vectors |
| Flags/no-op | CLC SEC CLI SEI CLV CLD SED NOP | Intended flag only, interrupt polling implications |

**Accept:** every official encoding has executable tests and the table has no unexplained coverage gaps.

### P40-03 — Address calculation and cycle details

1. Test zero-page indexing wraps at `$FF`, and zero-page pointer high-byte reads wrap within zero page.
2. Test absolute/indexed-indirect/indirect-indexed effective addresses at page boundaries and at the 16-bit address wrap.
3. Reproduce the NMOS indirect JMP page-boundary behavior: a pointer at `$xxFF` reads its high byte from `$xx00`.
4. Test relative branches from the PC after the operand, including negative displacement and a taken branch crossing a page. Record separate not-taken, taken-same-page, and taken-cross-page totals.
5. Verify indexed reads' conditional cycles and stores' fixed timing. Read-modify-write instructions require their real bus accesses, including dummy access/write behavior; do not add an undifferentiated “page-cross cycle” to every instruction.
6. Use a recording bus to check access order where a dummy read/write can trigger device side effects. Verify normal RAM cases and a peripheral-observable case.
7. Check total cycles advance consistently across `step_instruction`, frame execution, debugger pause, and resume. Do not account for the same DMA or interrupt cycles twice.

**Accept:** crossing and non-crossing cases produce correct addresses, values, and cycle/access sequences. Use [addressing-mode reference](https://www.nesdev.org/wiki/Addressing_modes) and the selected core's per-cycle implementation as review aids.

### P40-04 — ALU flags and stack correctness

1. Test ADC and SBC over all 256×256 operand pairs and both carry inputs where practical. Compute expected binary results/flags independently of the production helper.
2. Add focused signed-overflow cases (`$7F+1`, `$80-1`), borrow/no-borrow, result zero, and negative results. Confirm SBC carry means no borrow.
3. Verify BIT derives Z from the masked result and N/V from the operand; compare operations must not overwrite the accumulator.
4. Test rotate through carry, shifts of `$00/$01/$80/$FF`, and accumulator versus memory behavior.
5. Test stack wrap within `$0100–$01FF`, nested JSR/RTS, PHP/PLP, and BRK/RTI. Separate internal status bits from the status byte pushed on the stack.
6. Verify the decimal flag does not enable BCD arithmetic on this CPU model.

**Accept:** arithmetic/status edge cases and stack effects pass independently of nestest.

### P40-05 — RESET, NMI, IRQ, BRK, and DMA interaction

1. Test normal RESET through vector `$FFFC/$FFFD`; distinguish power-on fixture initialization from a later hardware reset. Verify the supported reset sequence, interrupt-disable state, stack effects, and cycles.
2. Test NMI vector `$FFFA/$FFFB` and IRQ/BRK vector `$FFFE/$FFFF` using cartridge-backed vector reads.
3. For BRK verify the two-byte return-PC convention and pushed status B bit. For hardware IRQ/NMI verify the saved status differs appropriately. Confirm RTI restores status and PC.
4. Model/test NMI as an edge-triggered pending event and IRQ as a level-sensitive request subject to the interrupt-disable flag. Test multiple interrupt sources without one device clearing another's pending request.
5. Test NMI priority, masked IRQ, held IRQ after RTI, and IRQ polling around CLI/SEI/PLP/RTI. Include the selected CPU model's documented instruction-boundary delays.
6. Use cycle-scheduled fixtures for interrupt arrival during BRK/IRQ entry, including NMI hijacking. Test these at the CPU/bus level without waiting for a future PPU implementation.
7. Verify existing OAM DMA stalls and CPU resumption, including the appropriate alignment-dependent timing, and preserve the existing core's DMC DMA behavior. Full new APU synthesis remains phase 43 work.
8. Reset between tests and verify interrupt/DMA state does not leak across instances or sequential test runs.

**Accept:** vector, stack, priority, masking, and timing fixtures all pass. [CPU interrupts](https://www.nesdev.org/wiki/CPU_interrupts) documents the distinctions and interrupt-entry edge cases; retain exact test expectations in source.

### P40-06 — Unofficial opcodes and jam behavior

1. Enumerate required unofficial encodings from the pinned nestest fixture and supplemental CPU suite. At minimum cover LAX, SAX, DCP, and ISC/ISB as requested by the roadmap.
2. Cover stable combined operations SLO, RLA, SRE, RRA, DCP, ISC; LAX/SAX; unofficial NOP variants; and alternate SBC where the selected suite exercises them. Preserve correct instruction lengths, memory operations, flags, and cycles.
3. Record behavior for other undocumented operations (such as ANC, ALR, ARR, AXS/SBX, LAS and unstable stores) in the coverage table. Test the chosen NES-core behavior and explicitly label hardware-dependent cases; do not claim full silicon equivalence solely from nestest.
4. Define KIL/JAM stop behavior as an explicit CPU result and verify reset recovery. Do not implement every unknown byte as a two-cycle NOP.
5. Test unofficial RMW operations' bus activity, not only their final arithmetic result.

**Accept:** every unofficial opcode used by the selected acceptance fixtures passes, all 256 entries have a deliberate classification, and jams terminate bounded execution safely. Reference [unofficial opcode encodings](https://www.nesdev.org/wiki/CPU_unofficial_opcodes).

### P40-07 — Pin the nestest ROM and canonical log

**New/proposed:** `emulator/tests/fixtures/nestest/manifest.json`, fixture README, ROM/log files or verified acquisition script.

1. Obtain Kevin Horton's nestest test ROM and the canonical `nestest.log` from a documented source, such as the [nes-test-roms collection](https://github.com/christopherpow/nes-test-roms/tree/master/other). Pin a source commit, not `master` at test time.
2. Record source URL/revision, file names, SHA-256 for both ROM and log, log record count, initial registers/cycles, and terminal boundary in a manifest. Calculate hashes from actual downloaded bytes; do not copy guessed values from this handoff.
3. Preserve the test asset's applicable distribution/readme terms. Commit redistributable fixtures or provide a hash-verified setup command and make missing fixtures fail the acceptance job. Normal tests must not fetch changing remote content.
4. Inspect the log format. The commonly used canonical log has 8,991 records; verify the selected file's exact count and final record rather than relying on a hard-coded count alone.
5. Do not substitute TetaNES's shipped `nestest.txt` without verification: the inspected dependency file uses a different rendering of flags/PPU coordinates. It is useful supporting evidence, not an automatically interchangeable golden file.
6. Include a short fixture-provenance README explaining trace timing and why automatic entry differs from booting the interactive ROM normally.

**Accept:** a clean checkout/setup yields a known ROM/log pair with verified hashes and immutable expected metadata.

### P40-08 — Implement the strict golden comparison

**New/proposed:** `emulator/tests/nestest_test.rs`, structured log parser/comparator and CLI from P40-01.

1. Load the ROM using phase 39's production cartridge/bus route. Configure deterministic state and the automatic test entrypoint.
2. For the usual canonical profile, initialize **PC=$C000, A=X=Y=$00, P=$24, SP=$FD, CPU cycles=7**, confirming these against the chosen manifest. Set this only in the test profile; normal ROM reset still uses its reset vector.
3. Parse the golden records into typed fields. Do not rely on whitespace alignment or disassembler comments. Parse PC/A/X/Y/P/SP as hexadecimal and CPU cycles as decimal, rejecting missing/malformed fields.
4. Capture state **before** each instruction, compare PC/A/X/Y/P/SP/cumulative CPU cycles, then execute one instruction. Also compare opcode bytes to detect a wrong bank/load when possible.
5. Require exact field equality. A documented fixed origin convention must be resolved at initialization; do not permit per-line cycle offsets, ignored flags, skipped addresses, or dynamically resynchronized traces.
6. If the selected log includes PPU coordinates, record their convention separately. CPU-cycle equality is mandatory here; do not fake a PPU model by displaying `cycles*3` and call that phase 41 accuracy.
7. On first mismatch, fail with index, expected/actual fields, current opcode/address, cycle delta, and the preceding bounded trace window. Exit nonzero from the CLI and test.
8. Require the full manifest record count and terminal boundary. Empty logs, premature EOF, a shorter generated trace, unexpected extra records under the selected stop policy, and budget exhaustion must fail. Never let `zip(actual,expected)` silently accept the shorter sequence.
9. Execute the final recorded instruction under the documented stop rule and verify the fixture's terminal result/signature where specified. Passing just the common prefix is not acceptance.
10. Test the comparator itself by deliberately altering one PC, register, status byte, cycle value, and line count; each case must fail. Add malformed/empty/missing-file cases and one known-good short parser fixture.
11. Save a machine-readable summary with ROM/log hashes, implementation revision, records compared, final state, and zero mismatches. Generated logs may be CI artifacts; account for the repository's `**/*.log` ignore rule if a golden log is tracked.

**Accept:** the entire pinned canonical log matches exactly, and mutation tests prove the runner detects discrepancies.

### P40-09 — Fix discrepancies in a reproducible order

1. If the first record fails, verify ROM hash, header/trainer offsets, NROM mapping, automatic entrypoint, RAM initialization, registers, and cycle origin before changing instruction code.
2. If PC diverges, inspect the preceding instruction's size, branch target, stack return, vector, and mapper identity.
3. If registers or flags diverge, isolate the instruction with its preceding state into a small regression test and check addressing/ALU semantics.
4. If only cycles diverge, inspect branch/indexed penalties, dummy accesses, interrupt entry, and DMA accounting. Compare the preceding instruction because traces describe its input state.
5. If the failure is confined to unofficial opcodes, verify opcode alias, addressing form, RMW order, and carry propagation.
6. Patch the maintained core source/upstream fork with a focused test and pin the changed revision. Re-run the small reproducer, full nestest, then affected compiler/browser acceptance.
7. Do not fix only the standalone runner while leaving the browser on another core version. Check Cargo's resolved graph and generated WASM identity.

**Accept:** every applied correction has a small regression and the browser/native implementations remain the same.

### P40-10 — Supplemental verification and final CPU gate

1. Run official/unofficial opcode, addressing, interrupt, and bus-cycle tests beyond nestest. Use a NES/Ricoh-compatible subset of [SingleStepTests/65x02](https://github.com/SingleStepTests/65x02) or equivalent fixtures, with provenance and exact variant recorded; a generic 6502 decimal-mode expectation is not a valid 2A03 failure.
2. Add CPU interrupt/dummy-access ROMs or deterministic direct tests for behaviors nestest does not exhaust. Separate CPU correctness from PPU/APU-dependent suite failures; do not label skipped CPU requirements as passes.
3. Run the CPU tests natively on the host CI matrix, then run deterministic instruction/trace equivalence checks in the WASM/browser build. Verify counter widths and serialization do not change results.
4. Run compiled SwissBASIC math, calls, controller, scrolling, and audio fixtures again. Include a mapper-1 banked fixture and source breakpoint checks.
5. Confirm the release browser remains responsive for invalid ROMs, JAM, budget exhaustion, repeated reset, and debug tracing disabled.
6. Attach the full canonical comparison summary, coverage table, supplemental test report, and native/WASM identity to phase 40 acceptance.

**Phase 40 done:** all P40 tasks pass, including the full golden log with no ignored mismatches. Remaining PPU/APU/mapper accuracy work is recorded against its later phase rather than hidden under a CPU completion claim.

### Commands the developer must provide by M7

The following interface is a **proposed deliverable**, not a command available at the audited commit. Implement it or document an equally explicit replacement in the final README. Configure Cargo binary/test names to match the published commands.

```text
cargo test -p swiss-emulator --test header_test --locked
cargo test -p swiss-emulator --test bus_test --locked
cargo test -p swiss-emulator --test nestest_test --locked
cargo run -p swiss-emulator --bin nes_trace --locked -- --rom emulator/tests/fixtures/nestest/nestest.nes --reference emulator/tests/fixtures/nestest/nestest.log --profile nestest --report target/nestest-report.json
```

The comparison command must run headlessly, produce a nonzero exit on mismatch, and work on Windows/macOS/Linux without shell-specific pipes. If a trace-only mode and separate comparator are supplied, provide a cross-platform launcher that checks both exit codes.

## 11. Release acceptance and handover

### 11.1 Final integrated demonstration

Create an authored `phase40_acceptance` example outside git-ignored user projects, with a documented import/copy operation into the IDE. Use original simple art/audio.

1. Open a fresh checkout, execute documented setup, and start the server.
2. Create/import the example with Main plus an included file, WORD math, an enum state machine, a macro, and a structured player.
3. Edit/save/reload source and assets. Confirm both files and all asset references persist.
4. Compile and download the ROM and source map. Verify the ROM header, artifact hashes, and asset/memory placement summary.
5. Run it in the browser; move the animated player with keyboard and a physical controller.
6. Cross horizontal and vertical map boundaries, collide with a tile/object, spawn/despawn pooled entities, and show text using the font fixture.
7. Play music/envelopes/arpeggio, trigger a higher-priority SFX and a DPCM sample, and verify the intended priority behavior.
8. Pause, reset, scale, fullscreen, adjust volume, close/reopen, and load again.
9. Set a breakpoint in the included file; stop before its side effects. Inspect registers/user RAM, CHR, nametables, palettes, and an offscreen sprite. Continue and hit the breakpoint again.
10. Run the headless phase 39 fixtures and phase 40 canonical log command from a fresh terminal. Preserve the generated report.

Use multiple small linked fixtures if the entire demonstration exceeds the compiler's documented ROM/RAM limits. Do not weaken placement checks or silently drop assets to fit a showcase.

### 11.2 Required final checks

```text
cargo check --workspace --all-targets --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo check -p swiss-emulator --target wasm32-unknown-unknown --locked
git diff --check
```

Also run the pinned WASM artifact generation, browser automation command, canonical comparison, and host CI matrix. Publish the exact browser command after R01 establishes its harness; do not leave placeholders in the final developer README.

### 11.3 Required handover files and sign-off

1. Updated `README.md` with clean setup, run/test commands, WASM packaging, example usage, and actual supported limits.
2. Updated `DESIGN.md` marking only proven phases complete; explain the addressing-mode counting convention without changing the phase's intended CPU coverage.
3. Updated `AGENTS.md` Brain with actual architecture, memory map, known limitations, test commands, and phase 41 as the next roadmap phase only when this acceptance is complete.
4. Architecture record, mapper contract, cartridge format support matrix, CPU opcode coverage table, and fixture provenance/hashes.
5. A completed `docs/PHASE_40_ACCEPTANCE_CHECKLIST.md` with commit/test/evidence links for every phase and 25 subphase.
6. CI artifacts containing test results, screenshots/audio evidence where applicable, canonical comparison report, and built browser assets tied to a commit.
7. A short release note listing user-visible repairs, schema migrations, compatibility limits, and any later-phase backlog. Do not leave missing required phase functionality in that backlog while marking it complete.

### 11.4 Suggested review-sized delivery sequence

| PR | Deliverable | Review gate |
|---|---|---|
| 1 | Baseline documentation, workspace/native/WASM CI | Failures and package coverage reproducible |
| 2 | API compile regression and runtime path repair | Minimal and feature-specific API fixtures boot |
| 3 | Bank/RAM placement, startup, cross-bank correctness | Boundary and reset/call fixtures pass |
| 4 | Source-map schema and provenance | Byte-accurate file/bank/range tests pass |
| 5 | Native/runtime and browser harnesses | Deterministic tests exercise production code |
| 6 | Language phase 1–10 repairs and acceptance | All ten phase evidence rows complete |
| 7 | Game-runtime phase 11–20 repairs and acceptance | Input/render/gameplay evidence complete |
| 8 | Audio phase 21–25d repairs | Audio schema and playback round-trips pass |
| 9 | Visual phase 26–30 repairs | World map bytes and animations work in ROM |
| 10 | WASM lifecycle/input/pacing, phases 31–34 | Fresh browser build and lifecycle tests pass |
| 11 | Breakpoints/memory/PPU views, phases 35–38 | Correct debug identity and buffer tests pass |
| 12 | Phase 39 cartridge metadata/parser | Format and malformed-input fixtures pass |
| 13 | Phase 39 mapper/bus integration | Actual core uses tested abstraction |
| 14 | Phase 40 native trace and pinned fixtures | Runner/parser failure tests pass |
| 15 | CPU coverage, timing/interrupt/unofficial repairs | Full nestest plus supplemental tests pass |
| 16 | Clean-checkout release acceptance and docs | All sign-offs complete |

This table describes review boundaries, not a fixed estimate. Split a row further when needed; maintain the milestone dependencies. R05 can be introduced incrementally before its standalone cleanup PR to support earlier repair tests.

## 12. References

The local source audit is the basis for repository-specific findings. Use these technical sources for implementation and fixture provenance, pinning versions where appropriate:

- [Audited SwissArmyNES commit](https://github.com/kd7tck/swissarmyNES/tree/4610516840d7991bce7e1d879e1f4e2fa831f10e): roadmap, source, tests, generated artifacts.
- [TetaNES upstream](https://github.com/lukexor/tetanes): dependency ownership and upstream changes. Audit behavior against the locked 0.12.2 source or the explicitly chosen patched revision, not the current upstream README alone.
- [NES 2.0 specification](https://www.nesdev.org/wiki/NES_2.0): cartridge size and metadata encodings.
- [iNES format](https://www.nesdev.org/wiki/INES): legacy header and payload conventions.
- [CPU memory map](https://www.nesdev.org/wiki/CPU_memory_map): bus routing.
- [CPU addressing modes](https://www.nesdev.org/wiki/Addressing_modes): address and timing behavior.
- [CPU interrupts](https://www.nesdev.org/wiki/CPU_interrupts): interrupt entry and timing edge cases.
- [CPU unofficial opcodes](https://www.nesdev.org/wiki/CPU_unofficial_opcodes): opcode classifications.
- [nestest collection and reference log](https://github.com/christopherpow/nes-test-roms/tree/master/other): choose and pin the ROM/log pair; this handoff does not bundle or assert a passing canonical run.
- [SingleStepTests 65x02](https://github.com/SingleStepTests/65x02): supplemental instruction/bus test data; select the CPU variant deliberately.

Some NESdev pages blocked direct retrieval during this audit; indexed source excerpts and the locally downloaded locked core were used where available. Verify the complete relevant format/timing specification during implementation and record it beside the fixtures. No future behavior or unexecuted test in this document is represented as an audit pass.

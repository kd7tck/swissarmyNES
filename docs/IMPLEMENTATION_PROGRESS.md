# Dev Loop 5 — PPU Viewer Attribute Grid Formatting Helper & JS Unit Tests, September 16, 2026

- Added `PpuViewer.formatAttributeGrid(paletteData)` static helper function to `static/js/ppu_viewer.js`.
- Added unit tests in `tests/js/ppu_attribute.test.cjs`.
- All Node JS unit tests pass.

# Dev Loop 4 — Cartridge Header Helper Methods & Battery Flag Verification, September 16, 2026

- Added `total_prg_ram()` and `has_battery_backup()` helper methods to `CartridgeInfo` in `emulator/src/cartridge.rs`.
- Added test coverage in `emulator/tests/cartridge.rs`.
- All `swiss-emulator` tests pass.

# Dev Loop 3 — Add Math.Wrap and Math.Lerp Intrinsics, September 16, 2026

- Added `Math.Wrap(val, min, max)` and `Math.Lerp(a, b, t)` intrinsics in `src/compiler/analysis.rs` and 6502 assembly codegen in `src/compiler/codegen.rs`.
- Added test coverage in `tests/math_intrinsics_test.rs`.

# Dev Loop 2 — Add Memory Copy Intrinsic (Memory.Copy), September 16, 2026

- Added `Memory.Copy(src_address, dst_address, length)` intrinsic support in `src/compiler/analysis.rs` and 6502 pointer block copy loop codegen in `src/compiler/codegen.rs`.
- Added integration test `test_memory_copy_codegen` in `tests/memory_intrinsics_test.rs`.

# Dev Loop 1 — Add Bitwise Shift Intrinsics (BITSHL, BITSHR), September 16, 2026

- Added `BITSHL(val, count)` and `BITSHR(val, count)` bitwise shift intrinsics with constant folding, semantic analysis, and 6502 assembly code generation in `src/compiler/analysis.rs` and `src/compiler/codegen.rs`.
- Added test coverage in `tests/bitwise_shift_test.rs`.

# Remote branch merge and Memory.Fill fix — September 16, 2026

- Refreshed GitHub refs and merged both additional remote branches (`dev-loops-intrinsics-diagnostics-13998583794852208848` and `jules-11030633264594705231-c27375b8`) into the updated `main` at `6134fa4`.
- Reconciled overlapping Math intrinsic implementations so `Math.Abs`, `Math.Min`, `Math.Max`, `Math.Sign`, and `Math.Clamp` remain available together, along with the mapper, frontend, and diagnostic work from both branches.
- Fixed `Memory.Fill` overwriting its length when an indexed destination generated address scratch code in `$00/$01`; the length now survives on the stack until the destination address is complete. Added an indexed-destination regression test.
- Native workspace tests, strict Clippy, formatting, WASM rebuild, and all frontend Node tests pass after the fix. Phase acceptance remains incomplete.

# Dev Loop 5 — Add Controller.AnyPressed Intrinsic, September 16, 2026

- Added `Controller.AnyPressed()` intrinsic support in `src/compiler/analysis.rs` and `src/compiler/codegen.rs`.
- Added integration test `test_controller_any_pressed` in `tests/controller_test.rs`.

# Dev Loop 4 — Array Out-Of-Bounds Constant Index Error Diagnostics, September 16, 2026

- Added semantic validation in `src/compiler/analysis.rs` reporting errors for array indexing with out-of-bounds constant indices.
- Added integration test `test_array_out_of_bounds_constant` in `tests/array_test.rs`.

# Dev Loop 3 — Constant Folding for Bitwise Operations, September 16, 2026

- Extended `fold_constants_expr` in `src/compiler/analysis.rs` to fold `BITAND`, `BITOR`, `BITXOR`, and `BITNOT` expressions at compile time.
- Verified with unit tests in `tests/foundation_test.rs`.

# Dev Loop 2 — Add Math.Sign Intrinsic Alias, September 16, 2026

- Added `Math.Sign(val)` intrinsic validation in `src/compiler/analysis.rs` and 6502 assembly generation in `src/compiler/codegen.rs`.
- Added integration test `test_math_sign_generation` in `tests/math_advanced_test.rs`.

# Dev Loop 1 — Add Math.Clamp Intrinsic, September 16, 2026

- Added `Math.Clamp(val, min, max)` intrinsic validation in `src/compiler/analysis.rs` and 8-bit/16-bit 6502 assembly generation in `src/compiler/codegen.rs`.
- Added integration test `test_math_clamp_generation` in `tests/math_advanced_test.rs`.

# Latest checkpoint — Mapper Trait Abstraction and Cartridge Header Testing, September 16, 2026
# Latest checkpoint — Loop 5: Frontend Debugger Enhancements & JS Unit Tests

- Enhanced PPU Debugger OAM formatting in `static/js/ppu_viewer.js`.
- Added OAM entry formatting test coverage in `tests/js/ppu_viewer.test.cjs`.
- All native Rust workspace tests and Node.js frontend tests pass.

# Earlier checkpoint — Loop 4: UxROM (Mapper 2) Support in Emulator Engine

- Verified UxROM (Mapper 2) switchable $8000-$BFFF bank and fixed $C000-$FFFF bank execution.
- Added integration test `test_uxrom_mapper_2_bank_switching` in `emulator/tests/mapper_test.rs`.
- All emulator tests pass.

# Earlier checkpoint — Loop 3: Compiler Memory Utility Intrinsic (`Memory.Fill`)

- Added `Memory.Fill(address, length, value)` intrinsic in `src/compiler/analysis.rs` and `src/compiler/codegen.rs`.
- Generates efficient 6502 RAM fill loops. Added test coverage in `tests/memory_intrinsics_test.rs`.

# Earlier checkpoint — Loop 2: Compiler Math Intrinsics (`Math.Abs`)

- Added `Math.Abs` intrinsic in `src/compiler/analysis.rs` and `src/compiler/codegen.rs`.
- Supports 8-bit and 16-bit signed operands, generating 6502 assembly sign checks and two's complement negations.
- Added tests in `tests/math_intrinsics_test.rs`.

# Earlier checkpoint — Loop 1: Cartridge Header Parsing & NES 2.0 Hardening

- Hardened and tested `CartridgeInfo::parse` in `emulator/src/cartridge.rs` and `emulator/tests/cartridge.rs`.
- Verified validation for invalid signature ("NES\x1a"), corrupted header variant flags, truncated headers, zero PRG ROM, and invalid NES 2.0 RAM sizes.
- All emulator tests pass.

# Earlier checkpoint — Mapper Trait Abstraction and Cartridge Header Testing, September 16, 2026

- Implemented the `Mapper` trait in `emulator/src/cartridge.rs` (`read_prg`, `write_prg`, `read_chr`, `write_chr`, `step_irq`) and implemented it for `Emulator` in `emulator/src/lib.rs`.
- Added `emulator/tests/mapper_test.rs` integration tests to exercise `read_prg`, `write_prg`, `read_chr`, `write_chr`, and `step_irq` via the `Mapper` trait.
- Expanded `emulator/tests/cartridge.rs` unit tests to cover 12-bit mapper IDs and submapper extraction under NES 2.0, four-screen and battery flags, and RAM/NVRAM size shift decoding.
- All native emulator tests, strict workspace Clippy, formatting checks, and frontend JS Node tests pass.

# Latest checkpoint — trainer reset fix, September 15, 2026

User requested a concrete short implementation before stopping at low credits (last reported 6%). No commit. Source/build/test-only final commit policy remains unchanged.

- Fixed trainer boot data being lost on Emulator::reset (hard reset). Before the fix, the regression observed random RAM at $7000 instead of trainer byte $5A.
- Emulator now retains the 512-byte trainer only after successful transactional ROM load, replaces/clears it on subsequent successful loads, and reapplies it through the actual CPU bus after hard reset. Failed replacement leaves the loaded cartridge and its trainer intact.
- Regression checks all 512 bytes after reset, including a failed replacement first, then executes the ROM and verifies its read of restored trainer data.
- Entire native swiss-emulator suite, strict workspace/all-target Clippy and formatting pass. Rebuilt WASM and regenerated ignored bindings; all five Node/frontend/WASM tests pass, including the complete canonical trace. Full compiler/workspace suite last passed immediately before this isolated emulator fix and was not rerun this turn. All jobs completed.
- Core preparation's real-path containment guard was also exercised successfully in the preceding quick check. Run node scripts/prepare-core.mjs before Cargo on fresh checkouts; see previous checkpoint and README.

Remaining phase acceptance is unchanged except trainer reset policy now implemented: reset is a fresh boot and restores the original trainer image. Separate RAM/NVRAM persistence, other mapper RAM defaults, full browser integration, active NMI workloads and other phase gates remain pending. Stop here; resume only when user asks.
# Latest checkpoint — low-credit pause, September 15, 2026 (patched core)

**User requested another low-credit pause. No commit. Final commit must contain source/build/test requirements only. Generated artifacts, downloaded fixtures, generated .tools dependency sources and local checkpoint documents must stay excluded.**

Resume prerequisite: `node scripts/prepare-core.mjs` reconstructs ignored `.tools/tetanes-core` from checksum-pinned upstream crate plus source patch. It must run before Cargo on a fresh checkout and after patch edits, with no active build. README and both CI jobs were updated. See the next section for exact patch, checksum and semantics.

Latest verified results:
- Entire native swiss-emulator suite passed against patched core: cartridge, CPU trace/status, interrupt, debugger and palette tests.
- Strict workspace/all-target Clippy and formatting passed.
- Rebuilt release WASM and regenerated ignored browser bindings. All five Node tests pass, including all 8,991 canonical CPU records and new actual WASM NROM capacities (none, 128 bytes, 8 KiB NVRAM).
- Source-only Git check found no tracked WASM/EXE/DLL/NES/ZIP files. The five original generated WASM files remain staged for removal; no other final staging/commit has occurred.
- Full native workspace/all-target locked/offline suite running in exec session 72562 when written, output docs/latest-native-test.log (ignored). Record final result before leaving or consult the update below.
- Added a final path-containment check to prepare-core.mjs: build storage must resolve to a real .tools directory inside the project. Node syntax check passed; full preparation had passed twice before this guard was added. Do not rerun preparation during a build. Network fallback remains untested because the archive was available in Cargo cache.

Remaining limitations: combined backend RAM/NVRAM capacity has no separate persistence/reset policy yet; other mapper defaults are still unresolved. NROM no-CHR-ROM needs explicit 8 KiB CHR RAM; >8 KiB PRG RAM rejected. Uniform trainers with absent RAM rejected before open-bus readback. Phase 39/40 and earlier integration gates remain incomplete. Continue from these specific gaps and the preceding handoff checklist; do not mark completion or commit just because focused tests pass.
# Patched core / NROM RAM semantics — September 15, 2026

User resumed implementation. No commit. Final commit remains source/build/test requirements only, with generated dependency sources, binaries, downloaded fixtures and local checkpoints excluded.

## New prerequisite

Run `node scripts/prepare-core.mjs` before Cargo on a fresh checkout (and after changing the patch, while builds are stopped). Root Cargo.toml patches tetanes-core to ignored `.tools/tetanes-core`, excluded from workspace membership. Cargo.lock now resolves that local source package.

The script verifies the official tetanes-core 0.12.2 crate SHA-256 `5ab83febf2a67da4ec29d509e9848c18109a7a564f0472a98cdc5cea7c701f47`, taking it from Cargo's archive cache or downloading the pinned archive. It extracts into ignored staging, checks/applies `patches/tetanes-core-0.12.2.patch`, and only then replaces the generated dependency folder. No upstream binaries are committed. Git and tar are prerequisites. `.gitattributes` forces LF for patch files (a CRLF last context line initially broke patch application; normalized LF fixed it). Preparation from cached archive was tested twice successfully; network fallback has not been exercised here. Failed preparation staging directories may remain under ignored .tools/core-prepare-*.

## Changes

- Backend Cart RAM size calculation now decodes the independent low/high nibbles and combines volatile/nonvolatile capacity. Previously the entire byte was treated as one shift, rejecting e.g. nonvolatile-only 0x70.
- Backend NROM honors explicit NES 2.0 PRG RAM absence and capacity. It returns the bus for absent RAM and mirrors smaller declared RAM within the unbanked window. Legacy NROM retains its old 8 KiB default.
- Emulator exposes actual prg_ram_len for diagnostics/tests. NROM loads reject >8 KiB PRG RAM and CHR-ROM-less headers without an explicit 8 KiB CHR RAM/NVRAM declaration. A trainer with explicitly absent PRG RAM is rejected before open-bus readback could falsely appear to verify uniform bytes.
- Executed tests cover no-RAM open bus, 128-byte and 2 KiB mirrors, 8 KiB NVRAM and mixed volatile/nonvolatile combined capacity. These do not prove separate persistence semantics: bus storage is still combined and hard-reset/battery policies remain open. Other mappers' defaults and NES 2.0 edge cases remain unfinished.
- README and native/WASM CI now prepare the patched dependency. Source patch documentation is in patches/README.md.

## Verification

- cargo check and strict workspace/all-target Clippy pass.
- Entire swiss-emulator native suite passes: 7 cartridge tests, 6 interrupt tests, 4 CPU trace/status tests, debugger and palette checks. Canonical nestest still matches all 8991 records.
- WASM rebuild and Node verification running in exec session 14323 when this checkpoint was written. Record final result below. Full compiler/workspace suite against the patched core still needs a run.

Remaining phase gates from prior checkpoints still apply. No phase 39/40 completion claim.
# Latest checkpoint — low-credit pause, September 15, 2026

**User requested a pause and documentation before stopping. No commit. Resume from this section. Final commit must contain only source and build/test requirements; generated binaries, downloaded fixtures, logs and local checkpoint documents stay excluded. Previously tracked static/wasm outputs are staged for removal, with local copies retained and ignored.**

## Latest implementation

- Interrupt wrappers now preserve temporary ranges $00-$0F, $14-$17 and $F0-$F7, plus cross-bank return scratch $07F1 and A/X/Y. A shared 28-byte ROM address table drives sparse save/restore loops. Persistent controller history ($10-$13), text offset ($18), sprite state ($19-$1C), scroll ($E0/$E1), RNG ($E2/$E3) and PPU shadow ($F8) remain shared deliberately.
- Reproduced helper-argument corruption before the fix: NMI changed $14 from 80 to 17. The new execution regression now passes for both NMI and IRQ across all 12 newly protected bytes and verifies that a handler's persistent text-offset change remains visible.
- The first large fixture hit the existing fixed-bank C000-D000 code limit. Separate range loops were replaced with a sparse table; the test uses a compact inline-ASM clobber block rather than repeated POKE calls to fit. Fixed-bank code headroom remains tight; do not claim arbitrary larger handlers now fit. A demand-based runtime/helper layout is still worth addressing.
- Timing regression exposed idle Sound_Update running envelope work for inactive channels: idle NMI took 4,289 cycles, beyond NTSC vblank. Sound_Update now iterates the four channel blocks in one loop and skips inactive channels before note work and after a channel stops. This also saves ROM space.
- Idle NMI now measures 1,774 CPU cycles including interrupt entry, scratch handling and OAM DMA. Nominal NTSC vblank is roughly 2,273 cycles, leaving only about 499 cycles for additional work. Active envelopes, buffered VRAM writes and user handlers require separate budgeting; this is not full vblank acceptance.
- New channel execution regression checks alternating and all-active channel masks: every active timer decrements exactly once; all bytes in inactive channel state remain untouched. Zero-initialized inactive channels must not be interpreted as envelope 0.

## Verification

- All 12 cross-bank/interrupt integration tests pass, including prior atomic ON binding and every-instruction MMC1 interruption tests.
- Updated nmi_safety_test passes for the sparse 28-byte scratch loop.
- Strict workspace/all-target Clippy and cargo fmt pass.
- Full locked/offline workspace all-target suite is running in exec session 70106, output docs/latest-native-test.log (ignored). Its final status has not yet been recorded at the time this checkpoint was written. Check the appended result below before rerunning. No other build/test jobs are running.
- Compiler-only changes this turn: generated emulator WASM did not need rebuilding.

## Resume priorities

1. Confirm final full-suite result below or from docs/latest-native-test.log; repair any regression before new work.
2. Budget active audio/VRAM/user-handler NMI workloads and address limited fixed-bank code headroom. Scratch preservation does not make all shared runtime state reentrant: static SUB argument storage and multi-byte persistent state still need explicit ownership/constraints.
3. Continue phase 39 cartridge RAM/NVRAM and bus/mapper contracts; supplemental phase 40 reset/DMC/IRQ edge cases; source-map/download/lifecycle browser acceptance; and remaining earlier phase gates in the handoff/checklist.
4. Keep final commit source-only, explicitly selecting files. No commit until implementation and required checks are complete. All phases are not yet accepted.
# Dynamic interrupt binding — September 15, 2026

Final commit remains source/build requirements only; generated outputs and local checkpoints stay excluded. No commit yet.

- Implemented previously ignored ON NMI/IRQ DO statements. Semantic analysis rejects unknown events and handlers that are absent, not routines, or require arguments. Codegen checks handlers again for direct callers.
- Bound targets have immutable low/high address tables in fixed-bank ROM. SUB targets dispatch through existing bank-switch trampolines; INTERRUPT targets use their fixed-bank labels. Only referenced targets enter the table, including ON inside nested control-flow blocks. Maximum 255 distinct bound targets.
- ON publishes a one-byte selector at 07F8 (NMI) or 07F9 (IRQ). Zero retains the startup/default pointer at 07F4/07F6. Dispatch snapshots the selector in X, resolves its address through ROM into saved scratch 00/01 and jumps. Single-byte publication also allows handlers to rebind safely without a half-updated pointer.
- Native execution tests verify both NMI and IRQ calling bank-3 routines, rebinding from inside the handler to bank 5, completing Main and restoring bank 0. Invalid handler diagnostics pass. A deterministic interrupt-injection test covers before LDA, before STA and after STA of publication, using linker source-map location; it observes exactly the old or new handler.
- All nine cross-bank tests and strict workspace/all-target Clippy pass. Full locked/offline all-target suite is running; record final result below. Compiler-only changes do not require a new WASM emulator build.

Remaining: wider helper scratch ownership and interrupt/vblank cycle budgets, cartridge/bus contracts, supplemental CPU edge cases, frontend integration acceptance and other phase gates. No complete-phase claim.
# MMC1 interrupt recovery — September 15, 2026

Final commit remains source/build requirements only, after implementation and checks are complete. No commit yet; generated files remain ignored.

- Reproduced mapper corruption by injecting NMI immediately after the first serial PRG write while its handler called a subroutine in another bank. Main failed to finish.
- MMC1_SetPrgBank now saves the desired bank in $07F0, clears retry flag $07F2, resets the MMC1 serial register, writes all five PRG bits, and retries if an interrupt set the flag. Both interrupt epilogues set $07F2 before restoring registers. Handler bank calls restore the interrupted bank shadow; the interrupted setter repairs any partial serial sequence before returning to switchable code. X/Y remain untouched by the setter.
- The regression resolves the setter via generated trampoline machine code and injects each NMI/IRQ at every instruction boundary through the setter's RTS. It uses the pinned production ControlDeck CPU/bus/mapper, invoking its interrupt entry rather than substituting a CPU. All boundaries and all six bank execution tests pass. Existing PPU/APU interrupt tests separately exercise real arrival/polling.
- This relies on compiler-generated mapper writes and the fixed-upper-bank MMC1 mode. Arbitrary inline-ASM register writes are not synchronized. Additional interrupts can cause retries; vblank cycle budgeting and wider runtime scratch ownership remain open.
- Full locked/offline workspace all-target suite is running; record its final result below. No emulator source changed this turn, so the local generated WASM does not require rebuilding for this compiler-only fix.
# Interrupt scratch preservation — September 14, 2026

Source-only final commit policy remains in force. No commit yet; generated files stay ignored and their previously tracked versions are staged for removal.

- Reproduced a runtime interrupt corruption: Main held 93 in cross-bank return scratch $07F1, NMI wrote 17, and Main observed 17 after return. New regression failed before the fix and passes after it.
- Both generated NMI and IRQ wrappers now stack/restore $07F1 alongside saved registers and $00-$0F scratch. This isolates cross-bank expression return storage across interrupts.
- Added instructions initially pushed a larger nested-bank test into the fixed D000 data segment. Replaced duplicated unrolled 16-byte scratch saves/restores with indexed loops in both wrappers; all four cross-bank execution tests now pass. Loops add interrupt cycles and need vblank workload budgeting; this is not complete interrupt/runtime acceptance.
- Partially interrupted MMC1 serial writes, wider runtime scratch, and interrupt metadata/banked dynamic handlers remain unresolved. Do not report MMC1 reentrancy fully fixed.
- Full locked/offline workspace all-target suite and strict Clippy are running at this checkpoint. Record their actual final results before proceeding.
# Source-only commit policy and IRQ coverage — September 14, 2026

User explicitly requires the eventual commit to contain only source and materials needed to build; compiled outputs must be excluded. Commit only when implementation and checks are complete. Select paths explicitly: code, tests, build/CI scripts, dependency manifests/lockfile, ignore rules and build instructions. Local progress/audit/checkpoint documents are not final-commit material. No commit yet.

- Removed all five tracked generated static/wasm files from the Git index using git rm --cached; local copies remain. These removals are staged. static/wasm is now ignored. No tracked WASM/EXE/DLL/NES/ZIP files remain in the prospective tree.
- External nestest binary, trace and author readme are ignored. scripts/fetch-test-fixtures.mjs downloads pinned upstream inputs, verifies SHA-256 before writing, verifies existing inputs without a network call, and refuses checksum mismatches. Provenance stays in source documentation. The existing-input verification branch passed; fresh network download has not been separately exercised here.
- README documents pinned Rust/target/wasm-bindgen build and fixture/test steps. Native CI fetches fixtures and runs source frontend tests; WASM CI builds fresh bindings before the bundle trace test. No checked-in generated bundle dependency remains.
- New APU frame IRQ regression verifies I masking, CLI polling delay, cartridge IRQ vector/stack B=0, and immediate held-IRQ reentry after RTI. All six CPU interrupt integration tests pass. Existing five frontend/WASM tests pass with local generated artifacts.

Remaining phase requirements persist. Next: reset/DMC/interrupt priority edge cases, compiler interrupt/MMC1 reentrancy, bus and cartridge RAM contracts, browser integration. Do not commit on this checkpoint alone.
# CPU integration and regional pacing — September 14, 2026

Latest continuation: commit only when implementation and required checks are complete; no final diff tree. No commit yet.

- Five new production-ROM CPU regressions pass: exhaustive BRK status stacking and RTI (256 status values), page-one stack wrapping, indirect-JMP page wrap, PPU-triggered NMI/vector/stack/I masking/edge behavior, and OAM DMA 513/514-cycle alignment stalls. IRQ priority/polling, DMC, interrupt hijacking, reset distinctions and other phase gates remain incomplete.
- Added frame_rate() from active backend region and clock. Frontend pacing consumes it instead of NTSC constant. Native NTSC/PAL/Dendy ROM tests and 60/120/144Hz frontend timing tests pass. NTSC rate is nominal rendered-frame cadence (alternate one-dot skip); no-render frames have a tiny different cadence.
- Rebuilt WASM and regenerated bindings. New Node regression executes the exact browser bundle against all 8,991 canonical records: all PC/A/X/Y/P/SP/cycles match. It also checks palette length and frame rate. This validates WASM execution, not browser UI/audio integration.
- All five Node tests pass; strict workspace/all-target Clippy and formatting pass; targeted cartridge and CPU interrupt tests pass. CI now runs Node tests after freshly rebuilding WASM as well as testing the checked-in bundle in native jobs. Remote CI has not run.

Next: browser download/source-map/lifecycle proof, IRQ/DMC/reset supplemental coverage, mapper/bus architecture and RAM semantics, compiler interrupt/MMC1 reentrancy. Full phase acceptance remains pending.
# Delivery instruction updated — September 14, 2026

User explicitly changed final delivery: commit when implementation and required checks are complete. This supersedes all earlier requests for a final diff tree. Do not commit incomplete implementation. No commit has been made yet.

Latest CPU validation: four cpu_trace tests pass, including the complete 8,991-record trace, exhaustive PLP/PHP/RTI stack status checks, and comparator negative cases for every field and all eight status bits. Strict workspace/all-target Clippy passes. Phase 40 remains incomplete pending supplemental coverage and integration evidence.
# Implementation progress through phase 40

User instruction: implement the developer handoff and commit only when complete. All work remains uncommitted until the full acceptance gate passes. The handoff's suggested intermediate commits are superseded by that instruction.

## Execution plan

1. Restore production compilation, bank placement, and runtime execution tests.
2. Unify source maps and harden source/asset persistence and validation.
3. Verify and repair language, gameplay, audio, and visual authoring phases.
4. Rebuild WASM and verify browser lifecycle, input, and debugger behavior.
5. Complete cartridge/bus/mapper integration and CPU trace validation.
6. Run complete acceptance, update evidence, and only then commit.

## Current work

- Baseline documentation changes from the planning task are preserved.
- Added production HTTP/compiler regressions for the known failing path.
- No implementation phase has been signed off yet.

## Paused checkpoint

Paused at the user's request on September 13, 2026 due to low credits. Read [IMPLEMENTATION_RESUME.md](IMPLEMENTATION_RESUME.md) for exact implementation state, verification history, two currently failing later-phase tests, and ordered next steps. No commits were made and no phase is signed off.

## September 14 resumed work

- Fixed collision return-value corruption during stack cleanup (Rect/Point/Tile). Added miss cases; corrected controller fixture to supported syntax/API and verified release. All four phase_11_20_execution_test tests now pass.
- Unified legacy generate with generate_banks (flatten bank 0 and 7 only; other banks require bank-aware API). Complete workspace suite passed afterward; log: docs/latest-native-test.log (ignored).
- Added checked semantic struct/array sizing and exact 560-byte/overflow production allocation tests, passing.
- CI now defines Windows/Linux/macOS workspace checks and a pinned Rust 1.98.0 WASM build with wasm-bindgen-cli 0.2.106. CI has not run remotely.
- Installed wasm32 target and matching binding CLI in ignored .tools. Release WASM build and regenerated static/wasm succeeded.
- Debugger uses actual mapper PRG offset instead of guest bank-shadow RAM; stops before initial instruction; continue re-arms breakpoint. Added peek_cpu and prg_offset. ROM load is transactional and preserves configured sample rate; invalid rate ignored. emulator/tests/debugger.rs passes, including invalid-load preservation.
- Strict all-target workspace clippy passed before the last formatting-only and generated-binding updates. Later semantic and emulator changes have targeted passing tests; rerun final full gate after further changes.
- Browser verification succeeded using in-app browser: Run starter program opens emulator with Pause, no error/warning logs; Pause changes to Play. No full browser acceptance claimed.
- Current server is cargo run session 98588 on localhost:3000. Browser testTab is paused. Stop/restart server when Rust server code changes; frontend requires reload.
- Remaining roadmap and original checkpoint next steps still apply except repaired items above. No commit made.

## Updated delivery instruction — September 14

The user superseded the final-commit requirement: **do not commit**. After completing implementation, generate a final diff tree for handoff to another developer, including a patch, changed-file inventory, validation evidence, and application instructions. This changes delivery format, not implementation scope. No final diff package has been generated yet.

## September 14 further continuation

- Full locked native all-target workspace suite passed after the source-map/AST migration. Added and passed include/macro provenance test.
- app.js now requests both ROM and sourcemap.json downloads, discards superseded compile responses; project.js assigns currentFile before firing editor input. Editor maps are invalidated by project identity/content and project-loaded events.
- Added elapsed-time browser pacing with bounded catch-up and one scheduled RAF. Node tests at 60/120/144 Hz and duplicate/stalled callbacks pass. Added blur/hidden/close input cleanup and disconnect release. ROM reload now reuses transactional backend instead of freeing the old emulator first.
- Browser breakpoint verification succeeded: loop assignment line 4 receives `debug-active`, Play state indicates stopped, Continue hits it again. No download acceptance yet.
- Added checked cartridge metadata parser and narrow exponent/trainer adapter in actual loader. Four cartridge tests and existing debugger test pass. docs/EMULATOR_ARCHITECTURE.md records remaining limitations; no phase39/40 acceptance claim.
- Current server cargo run session 25604 is active; browser resumedTab is paused at a breakpoint. Generated WASM predates the latest cartridge changes and must be rebuilt before browser cartridge testing.

## Third pause

User requested another low-credit pause. Current canonical CPU trace fails at record104, status FF vs EF (B bit only). See top of IMPLEMENTATION_RESUME.md and emulator/tests/fixtures/PROVENANCE.md. Strict all-target workspace Clippy passes after the new test. Temporary browser closed; server already stopped. No commit or final diff package.
# Latest continuation — September 14, 2026

This section supersedes the older pause notes. User resumed implementation. No commits; final delivery remains a diff tree, not a commit. Phases 1–40 are not fully accepted.

- Resolved nestest record 104 mismatch as a debugger representation issue: B/U are not physical CPU flags. `get_cpu_state` now publishes U=1/B=0 while preserving all six real flags. Backend PLP retains pulled B/U; RTI clears both; PHP and interrupt pushes explicitly serialize them. Reference: https://www.nesdev.org/wiki/Status_flag . No backend execution patch or comparator masking was needed.
- All 8,991 canonical records match PC/A/X/Y/P/SP/cycles. Result RAM bytes are checked unchanged from initial random RAM: successful automation never writes/initializes $02/$03. The earlier zero assertion was invalid for production random RAM.
- Exhaustive 256-value PLP/PHP and RTI stack-status tests pass; RTI checks PC, SP and 28-cycle total. Supplemental IRQ/NMI/BRK/DMA and comparator negative tests remain.
- Palette export now exactly 128 bytes, laid out 16x2 RGBA, with 32-byte stack scratch. Frontend uses the explicit layout, validates length and renders pixelated swatches. Rust and Node regressions pass. Node regressions added to the host CI matrix.
- Validation: full locked/offline workspace all-target suite passed before palette edits; all emulator tests passed after palette edits; strict workspace/all-target Clippy and formatting passed; all 3 Node tests passed. WASM release rebuilt and bindings regenerated using CLI 0.2.106. Browser visual verification of the latest bundle remains.

Next: finish CPU trace comparator negative tests and supplemental phase-40 coverage, source-map/download/lifecycle browser proof, interrupt/MMC1 reentrancy, and the incomplete cartridge/bus contracts described below. Existing phase checklist stays pending where broader evidence is missing.






Verification update: the full locked/offline workspace all-target suite passed after updating the old unrolled-instruction NMI test for indexed loop bounds. Both real NMI and APU IRQ scratch-preservation regressions pass (five cross-bank tests total). An earlier concurrent build caused a Windows executable lock error; the final successful run was serialized. Runtime partial MMC1 serialization remains open. No commit.


Latest verification (September 15): full locked/offline workspace all-target suite PASSED with the MMC1 retry fix. All NMI/IRQ setter instruction-boundary injections pass. Dynamic ON NMI/IRQ DO binding was found parsed but ignored by both semantic analysis and codegen; implementing it needs validated zero-argument handler targets, fixed-bank trampolines, and interrupt-safe publication of the two-byte handler address. This is a confirmed next task, not implemented yet.


Verification result: full locked/offline workspace all-target native suite PASSED after dynamic interrupt binding. Strict Clippy, formatting and git diff whitespace checks pass. Prospective tracked tree still contains no WASM, EXE, DLL, NES or ZIP artifacts; five pre-existing generated WASM files remain staged for removal only. No commit.


Final pause verification: full locked/offline workspace all-target suite PASSED (exit 0). Strict Clippy, formatting and whitespace checks passed. All test/build processes completed. Paused as requested; no commit.




Final low-credit checkpoint: full locked/offline workspace all-target suite PASSED against patched core (exit 0). All jobs complete. Paused, no commit.


Quick final check: prepare-core.mjs completed successfully with its new real-path containment guard enabled, reconstructing the patched dependency from the verified cached archive. No implementation changes, build or test jobs started, or commits made. Stopped at user's low-credit request.

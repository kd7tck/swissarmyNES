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

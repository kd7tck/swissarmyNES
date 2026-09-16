# AI Agent Guidelines

This document serves as the primary instruction manual for AI agents working on the SwissArmyNES codebase. Strictly adhere to these guidelines to ensure consistency, quality, and stability.

## Core Directives

1.  **Strict Design Adherence**: The `DESIGN.md` file is the master roadmap. Do not deviate from the current phase or the specified architecture without explicit user approval.
2.  **Cross-Platform Compatibility**: This is a Rust application. It must compile and run on Windows, macOS, and Linux without modification. Avoid OS-specific commands or paths.
3.  **Documentation**:
    - Update `README.md`, `DESIGN.md`, and this `AGENTS.md` file whenever major changes occur.
    - Do not remove existing comments unless they are factually incorrect or the code they describe is deleted.
    - Add clear, concise comments to all new code, explaining the "why" more than the "what".
4.  **Testing**:
    - "Test Constantly" is the mantra. Run tests after every significant code change.
    - You are permitted to modify existing test files (`tests/`) if your changes legitimately break old tests (e.g., changing an API).
    - Create new tests for every new feature or bug fix.
5.  **Code Quality**:
    - **Formatting**: All code must pass `cargo fmt`.
    - **Linting**: All code must pass `cargo clippy -- -D warnings`. Address all warnings immediately.
    - **Dead Code**: While `dead_code` warnings might be suppressed in CI, strive to minimize unused code.

## Project Structure

-   `src/lib.rs`: The library entry point. Contains the compiler core and server logic modules.
-   `src/main.rs`: The binary entry point. Sets up the Axum server and handles command-line args.
-   `src/compiler/`: All compiler logic (Lexer, Parser, AST, Symbol Table, Codegen, Audio).
-   `src/server/`: Backend logic (API endpoints, project file management).
-   `static/`: The frontend (HTML, CSS, Vanilla JS).
-   `projects/`: Local storage for user projects (git-ignored, but structure matters).
-   `tests/`: Integration tests.

## Development Workflow

1.   **Read Brain section in AGENTS.md**: Pick back up where the last AI agent left off.
2.  **Plan**: Read the requirements, verify the state of the code, and formulate a plan using `set_plan`.
3.  **Edit**: Make changes.
4.  **Verify**:
    - Run `cargo check` to catch compilation errors early.
    - Run `cargo fmt` to fix style.
    - Run `cargo clippy` to ensure quality.
    - Run `cargo test` to verify functionality.
5.  **Refine**: If tests fail, diagnose and fix. Do not guess. Use `read_file` to see the actual code.
6.  **Store current state in Brain section of AGENTS.md**: Help the next AI agent out by telling them what has been done and what should be done next, allong with pitfalls to look out for.
7.  **Submit**: Only when all checks pass.

## Technical specifics

-   **Assembler**: The project uses `rs6502` for assembly generation. Be aware of its limitations (e.g., no `.BYTE` directive support, use `db` equivalent or injection).
-   **Frontend**: Plain HTML/JS/CSS. No build step (Webpack/Vite) is currently used. Keep it simple.
-   **Memory Management**: The NES has 2KB of RAM. The compiler must manage this strictly (`$0000-$07FF`).

## Brain
### Mapper Trait & Cartridge Header Testing — September 16, 2026

- Implemented the `Mapper` trait in `emulator/src/cartridge.rs` with `read_prg`, `write_prg`, `read_chr`, `write_chr`, and `step_irq` operations, implemented for `Emulator` in `emulator/src/lib.rs`.
- Added integration tests in `emulator/tests/mapper_test.rs` and unit tests in `emulator/tests/cartridge.rs` for submapper/12-bit mapper decoding, four-screen/battery flags, and RAM/NVRAM shift decoding. Native tests, Clippy, formatting, and WASM/Node JS tests pass.

### Trainer reset checkpoint — September 15, 2026

- CPU bus routing & mirroring tests (P39-06) added in `emulator/tests/bus_test.rs`. Verified 2KB internal RAM mirroring across $0800-$1FFF, PPU register mirrors, PRG-ROM write protection, and side-effect-free peeking. All native tests, Clippy, formatting, and WASM/Node JS tests pass.
- Concrete final low-credit fix: hard reset reapplies retained trainer bytes; failed replacement preserves boot data. Regression reproduced randomization before fix and now checks all 512 bytes plus ROM execution. Native emulator suite, Clippy, rebuilt WASM and five Node tests pass. See latest resume checkpoint; no commit, source-only policy unchanged. Stop until user resumes.

### Low-credit pause after core patch — September 15, 2026

- User requested pause. First resume section has current build prerequisites/results. Prepare-core source script and patch reconstruct ignored dependency; source-only commit policy unchanged. Native emulator suite, strict Clippy, regenerated WASM trace and NROM capacity tests pass. Check checkpoint for final full-workspace result; no commit.

### Patched core prerequisite — September 15, 2026

- Run node scripts/prepare-core.mjs before Cargo on fresh checkout. Cargo now resolves tetanes-core from ignored .tools/tetanes-core, reconstructed from verified upstream archive plus source patch. README/CI updated; source-only commit policy unchanged.
- NROM NES2 PRG RAM absence/mirroring and independent RAM nibble decoding fixed. Native emulator suite/Clippy pass; see latest resume checkpoint for WASM/full-workspace verification. Separate NVRAM persistence and other mapper defaults remain open. No commit.

### Low-credit pause — September 15, 2026

- User requested pause. Resume from the first section of docs/IMPLEMENTATION_RESUME.md. No commit; source-only delivery policy remains.
- Sparse interrupt scratch preservation and inactive audio-channel skipping implemented. Twelve bank/interrupt tests plus nmi_safety and strict Clippy pass. Idle NMI measures 1774 cycles including DMA; active workloads and tight fixed-bank headroom remain open. Check appended full-suite result in the checkpoint.

### Dynamic interrupt binding — September 15, 2026

- ON NMI/IRQ DO is implemented with validated zero-argument targets and atomic selectors at 07F8/07F9 into fixed ROM tables. Banked targets use trampolines; handlers can rebind. Nine bank/interrupt execution tests and strict Clippy pass; see latest resume checkpoint for full-suite result. No commit, source-only final policy unchanged.

### MMC1 interrupt recovery — September 15, 2026

- PRG bank setter now resets/retries interrupted serial writes using reserved 07F2. NMI/IRQ boundary-injection regression reproduced corruption before the fix and passes at every setter boundary after it. All six bank execution tests pass. Check newest resume section for full-suite evidence and remaining constraints. Source-only final commit policy remains; no commit yet.

### Interrupt scratch fix — September 14, 2026

- NMI/IRQ now preserve cross-bank return scratch 07F1. Reproduced failure before fix; four bank execution tests pass after. Compact indexed scratch loops avoid D000 runtime/data overlap but require vblank cycle budgeting. Partial MMC1 write reentrancy is still open. See latest resume checkpoint for full-suite result.

### Source-only delivery policy — September 14, 2026

- Final commit: source and build/test requirements only. No compiled/generated outputs, downloaded ROMs, archives, logs or local checkpoints. Explicit path selection required. No commit until full implementation/checks complete.
- Five generated static/wasm deletions are staged via git rm --cached; local files remain ignored. Fetch external fixtures using scripts/fetch-test-fixtures.mjs; README and CI updated. Six CPU interrupt integration tests now pass, including held APU IRQ and CLI delay. Read the newest resume section.

### CPU integration continuation — September 14, 2026

- Latest resume checkpoint records five new CPU integration regressions, backend-region frame pacing, and full canonical trace equivalence in the generated WASM. Five Node tests and strict Clippy pass. No phase signoff or commit yet; final delivery is commit when complete.

### Latest delivery instruction — September 14, 2026

- User superseded the diff-tree request: commit only when implementation and required checks are complete. No commit yet. Historical diff-tree instructions below are obsolete.
- CPU comparator negative tests now pass for every field and every status bit. See the newest resume checkpoint.

### Active continuation — September 14, 2026

- User resumed. See the first section of docs/IMPLEMENTATION_RESUME.md. All 8,991 canonical nestest records now match; exhaustive PLP/PHP/RTI status tests pass. Prior failing-trace notes below are historical.
- Exact 16x2 RGBA palette export/viewer implemented and tested; WASM regenerated. Full native suite passed before palette edits, emulator suite after, strict Clippy and three Node tests pass. Remaining phase gates and final diff delivery are incomplete; no commits.

### Third low-credit pause — September 14, 2026

- Latest state is the **first section** of [IMPLEMENTATION_RESUME.md](docs/IMPLEMENTATION_RESUME.md). User requested another pause; no commit, eventual delivery is a final diff tree.
- New canonical nestest test currently fails at record104: expected P=EF, observed P=FF, other fields match. Investigate the status B-bit representation; do not claim phase40 complete or silently weaken comparison.
- Source-map browser breakpoint/re-hit, cartridge/trainer execution, asset checks, deterministic compiler reuse and refresh-independent pacing have new passing regressions. Strict all-target Clippy passes. Full current suite is not green because of the CPU trace failure.

### Latest pause — September 14, 2026

- Read the **top section** of [docs/IMPLEMENTATION_RESUME.md](docs/IMPLEMENTATION_RESUME.md) first. User requested a second low-credit pause.
- Delivery changed: **generate a final diff tree, not a commit**. Nothing is committed; phase 40 remains incomplete.
- The two earlier execution failures are fixed. Production source maps now use a versioned object with exact source snapshots and linker-resolved ranges; frontend consumers were migrated but browser verification and sourcemap.json download remain pending.
- Native/WASM builds, debugger regressions, and targeted source-map/compiler tests have passed as recorded in the checkpoint. Do not equate those with complete phase acceptance.

### Implementation pause — September 13, 2026

- User requested a pause for low credits. Resume from [docs/IMPLEMENTATION_RESUME.md](docs/IMPLEMENTATION_RESUME.md), which records changes, verification, failing fixtures, and next steps.
- All work remains uncommitted by user instruction. Implementation through phase 40 is incomplete. The latest phase 11–20 execution tests have two unresolved failures; do not report full acceptance.
- The production linker now uses the rs6502 opcode catalog with its own data emission and branch relaxation; the older assembler limitation note above describes the upstream library.

### September 2026 audit and developer handoff

- Read [the phase 40 developer handoff](docs/DEVELOPER_HANDOFF_THROUGH_PHASE_40.md), [baseline audit](docs/BASELINE_AUDIT.md), and [acceptance worksheet](docs/PHASE_40_ACCEPTANCE_CHECKLIST.md) before resuming implementation.
- Audited commit: `4610516840d7991bce7e1d879e1f4e2fa831f10e`. Native workspace tests, formatting, and strict Clippy pass, but three production compile API probes return HTTP 400 assembler errors, including an empty Main. Restore that path first.
- The current DESIGN phase 39 is Repository Ingestion & Bus/Cartridge Architecture; phase 40 is CPU Core Hardening. The older MMC1 next-step note below is historical and superseded by the current roadmap.
- The notes below are prior implementation claims, not newly verified acceptance. This handoff work changed documentation only; no implementation phase was completed during the audit.

Historical implementation summary: phases 31-38 were marked complete.

### Phase 31: Emulator - WASM Integration (Completed)
- **Implemented**: `swiss-emulator` crate in `emulator/` directory using `tetanes-core`.
- **Compiling**: `emulator` compiles to `wasm32-unknown-unknown` and exposes `Emulator` class via `wasm-bindgen`.
- **Frontend**:
    - `editor.js` handles loading the WASM module dynamically.
    - `editor.js` creates a Canvas overlay when "Run (Emulator)" is clicked.
    - `app.js` listens for `request-compile-and-run` event, compiles source via API, and passes ROM bytes to `editor.js`.
- **Key Features**:
    - **WASM Module**: Exposes `load_rom`, `step`, `get_pixels`, `set_button`, `reset`, `set_sample_rate`, `get_audio_samples`.
    - **Input**: Maps Z/X (A/B), Shift (Select), Enter (Start), Arrows (D-Pad).

### Phase 32: Emulator - UI Wrapper (Completed)
- **Implemented**: Emulator Overlay in `editor.js`.
- **Features**:
    - **Controls**: Play/Pause button (toggles loop), Reset button.
    - **Scaling**: 1x, 2x, Fullscreen buttons.
    - **Volume**: Slider controlling Web Audio GainNode.
    - **Integration**: `app.js` now handles the "Run" button by compiling the project (without downloading) and dispatching `emulator-load-rom`.

### Phase 33: Emulator - Input (Completed)
- **Implemented**: Gamepad support in `static/js/editor.js`.
- **Features**:
    - **Gamepad Polling**: Checks `navigator.getGamepads()` every frame in `emulatorLoop`.
    - **Mapping**: Xbox A -> NES A, Xbox B/X -> NES B, Back -> Select, Start -> Start, D-Pad/Axes -> D-Pad.
    - **Input Mixing**: Keyboard and Gamepad inputs are merged (Logical OR), allowing simultaneous use.
    - **State Management**: Updates are only sent to WASM when the combined state changes to minimize overhead.
    - **Hot-plugging**: Detects connection/disconnection of gamepads.

### Phase 34: Debugging - Protocol (Completed)
- **Implemented**: WASM interface exposes `get_cpu_state()` (PC, SP, A, X, Y, Status, Cycles) and `get_wram()` (Pointer to RAM).
- **Frontend**: `editor.js` includes `getDebugState` and `getWRAM` helpers.
- **Verification**: Code exists and is verified via review. The `emulatorLoop` logs CPU state to console every 60 frames for basic verification.

### Phase 35: Debugging - Source Maps (Completed)
- **Implemented**:
    - `CodeGenerator` emits `SourceMap` (Line -> Address).
    - `estimate_size` logic refined to handle `($ZP),Y`, `($ZP,X)`, `DB`, and `WORD` correctly.
    - `editor.js` receives `sourceMap` and implements `showAddressTooltip` on line hover.
    - Tooltip shows the hex address (e.g., `$8010`) corresponding to the source line.
- **Verification**: Verified `estimate_size` logic against `rs6502` assembler behavior. Verified frontend logic via code review.

### Phase 36: Debugging - Breakpoints (Completed)
- **Implemented**: Breakpoint support in `editor.js` and `emulator` crate.
- **Backend**: `emulator/src/lib.rs` maintains a list of breakpoints and checks PC every instruction. Returns `true` from `step()` if hit.
- **Frontend**: `editor.js` toggles breakpoints by clicking line numbers, syncing with emulator via `add_breakpoint`/`remove_breakpoint`.

### Phase 37: Debugging - Memory Viewer (Completed)
- **Implemented**: Live RAM viewer in `editor.js`.
- **Backend**: `emulator` exposes `get_wram()` returning a pointer to WASM memory.
- **Frontend**: `editor.js` reads WRAM and displays first 2KB (Zero Page, Stack, RAM) in a hex table, updating periodically.

### Phase 38: Debugging - PPU Viewer (Completed)
- **Implemented**: PPU Visualization in `static/js/ppu_viewer.js`.
- **Backend**: `emulator` exposes `update_pattern_tables`, `update_nametables`, `update_palettes`, and `get_oam_data`.
- **Frontend**: `PpuViewer` class renders Pattern Tables, Nametables, and Palettes to canvases and lists OAM entries.

### Bug Fixes
- **Audio/Assembler**:
    - Implemented strict overlap detection in `Assembler`.
    - Implemented size limits in `compiler/audio.rs` for Music Data, Samples, SFX, and Envelopes.
    - Fixed Audio Compiler gap overflow: Gaps > 255 frames are split into multiple silence commands.
- **Compiler/Codegen**:
    - Fixed `estimate_size` for `DB` and `WORD` directives. Previously, `DB` with multiple bytes (e.g., `DATA 1, 2, 3`) was estimated as 2 bytes (or 3 default), causing source map drift. Now correctly counts comma-separated values.
    - Fixed `estimate_size` for Indirect Indexed (`($ZP),Y`) and Indexed Indirect (`($ZP,X)`) addressing modes. It previously returned 3 bytes. Now correctly returns 2 bytes.
    - Verified that `rs6502` does *not* optimize explicit `$0010` (4-digit) addresses to Zero Page, so existing `estimate_size` logic for absolute addresses remains correct.
    - Fixed `Pool.Despawn`: Arguments are now evaluated safely. Base address is protected on stack while Index is evaluated, preventing register clobbering.
    - Verified Memory Map consistency.
    - Fixed `Runtime_GetHeapSlot`: Corrected 16-bit address calculation for String Heap slots > 15. Previous 8-bit logic caused heap wrapping at 256 bytes.
    - **RAM Overflow Check**: Added explicit check in `allocate_memory` to error if user variables exceed the NES RAM limit (`$07FF`). Added `tests/ram_overflow_test.rs` to verify.

### Memory Map
- **$0000-$00FF**: Zero Page.
    - `$E0`: Scroll X, `$E1`: Scroll Y.
    - `$E2`-$E3`: Random Seed (LFSR).
    - `$F8`: PPU Ctrl Shadow.
- **$0100-$01FF**: Stack.
- **$0200-$02FF**: OAM (Shadow Sprites).
- **$0300-$037F**: Sound Engine State (128 bytes).
    - Stride 32 bytes per channel (0, 32, 64, 96).
    - Offsets: State(0), Inst(4), Prio(5), Vol(6), Pitch(9), Arp(13), Base(17), Duty(19).
- **$0380-$03BF**: VBlank Buffer (64 bytes).
- **$03C0-$05BF**: String Heap (512 bytes).
- **$05C0-$07FF**: User Variables (DIM).
- **$8000-$CFFF**: PRG-ROM (Code).
- **$D000**: NTSC Period Table.
- **$D100**: Music Data.
- **$D480**: DPCM Sample Table (Addr, Len).
- **$D500**: Nametable Data (Ends at $D900).
- **$D900**: SFX Table.
- **$DA00**: Envelope Data Table.
- **$E000**: Palette Data.
- **$E040**: DPCM Samples (Start).
- **$FF00**: Data Tables (Vectors pointers).
- **$FFFA**: Vectors (NMI, Reset, IRQ).

- **Pitfalls**:
    - **WASM Pixel Format**: `tetanes-core` frame buffer format needs verification (RGBA vs RGB vs Palette). Currently assuming pointer access is sufficient for raw rendering, but color mapping might be needed if it returns raw NES palette indices. Future phases should verify color correctness.
    - `Text.Print` writes directly to the PPU ($2006/$2007). Use `WAIT_VBLANK` before calling this to avoid visual glitches.
    - `True` is `$FF`. Check assumptions in assembly injections if they rely on `1`.
    - **Audio Labels**: When injecting assembly strings in loops or multiple blocks, ensure labels are unique or use local labels if assembler supports it. `rs6502` global label reuse caused "Branch too far".
    - **DPCM Alignment**: Samples must start on 64-byte boundaries. Compiler handles padding.
    - **OAM Overflow**: `Sprite.Draw` drops sprites if 64 limit reached. Enable `Sprite.SetFlicker(1)` to mitigate limits via cycling.
    - **SFX Sequences**: `SequenceCanvas` modifies arrays in place.
    - **Frontend Validation**: When importing JSON, always validate fields exist to avoid `undefined` crashes in the editor.
    - **CHR Import**: Requires a 128x128 PNG for full bank import. Alpha channel is treated as color 0 (transparent). Nearest neighbor matching uses the *current* 4-color palette, not the full NES palette, so ensure the correct sub-palette is selected before importing.
    - **16-bit Pointers**: When calculating addresses (like Heap Offset), always handle 16-bit Carry (`BCC +; INX`) for the High Byte.
    - **Gamepad Polling**: `navigator.getGamepads` returns a snapshot. It must be polled in `requestAnimationFrame`.
    - **Input State**: When modifying input logic, ensure keyboard and gamepad don't conflict (e.g., releasing a button on one device shouldn't clear the hold on the other). Use a "last sent state" tracker.
    - **WASM Crash**: If the emulator crashes (e.g. `CpuCorrupted`), the loop stops and input polling ceases. Ensure robustness or handle errors gracefully if you need input to restart.
    - **Assembler Overlap**: The Assembler now strictly enforces non-overlapping segments. If you encounter "overlaps with existing data", check your `.ORG` directives and injection sizes to ensure they don't collide.
    - **CodeGenerator Stack**: When evaluating arguments for subroutines or built-ins, complex expressions can clobber temporary registers (like `$02/$03` or `$06`). Use the Stack (`PHA`/`PLA`) to protect intermediate values.

- **Dev Loops (Math Intrinsics, Bitwise Ops, Constant Folding, Lexer Diagnostics, JS Editor Tests)**:
    - Added `Math.Min` and `Math.Max` intrinsic support across analysis and 6502 code generation (supporting 8-bit & 16-bit operands).
    - Added `BITAND`, `BITOR`, `BITXOR`, and `BITNOT` bitwise functions in analysis and codegen.
    - Implemented `fold_constants_expr` in `SemanticAnalyzer` for compile-time constant expression optimization.
    - Enhanced `Lexer` with column position tracking (`column: usize`) and line:column error formatting.
    - Expanded JS unit tests in `tests/js/editor.test.cjs` covering pause/unpause state transitions and emulator reset.

- **Dev Loop 1: Cartridge Header Parsing & NES 2.0 Hardening**:
    - Enhanced `CartridgeInfo::parse` unit tests in `emulator/tests/cartridge.rs` to verify edge cases including invalid magic signature, corrupted iNES variants, submapper masks, and NES 2.0 flags.

- **Dev Loop 2: Compiler Math Intrinsics (`Math.Abs`)**:
    - Implemented `Math.Abs` intrinsic in `src/compiler/analysis.rs` and `src/compiler/codegen.rs` supporting 8-bit and 16-bit signed operands. Added test coverage in `tests/math_intrinsics_test.rs`.

- **Dev Loop 3: Compiler Memory Utility Intrinsic (`Memory.Fill`)**:
    - Implemented `Memory.Fill(address, length, value)` intrinsic in `src/compiler/analysis.rs` and `src/compiler/codegen.rs`. Added test coverage in `tests/memory_intrinsics_test.rs`.

- **Dev Loop 4: UxROM (Mapper 2) Support in Emulator Engine**:
    - Verified and tested UxROM (Mapper 2) PRG bank switching support in `emulator/tests/mapper_test.rs`.

- **Dev Loop 5: Frontend Debugger Enhancements & JS Unit Tests**:
    - Enhanced `PpuViewer.formatOamEntries` in `static/js/ppu_viewer.js` and added JS unit test in `tests/js/ppu_viewer.test.cjs`.

- **Next Steps**:
    - Run pre-commit checks and submit completed work.

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
Phase 1-23 are complete. Phase 24 is Next.

### Phase 24: Audio - Arpeggios (Completed)
- **Implemented**: Arpeggio support for audio tracks.
- **Backend**:
    - `AudioTrack` updated to include `arpeggio_env` field.
    - `compile_audio_data` updated to inject Arpeggio ID into track header (Header size 6).
- **Codegen**:
    - **Memory Map Updated**:
        - Sound RAM expanded to `$0300-$034F` (80 bytes).
        - String Heap moved to `$0350-$044F` (256 bytes).
        - VBlank Buffer moved to `$0450-$048F` (64 bytes).
        - User Variables start at `$0490`.
    - `Sound_Init`: Clears extended sound RAM range ($0300-$034F).
    - `Sound_Play`: Reads Arpeggio ID and resets Arp state.
    - `Sound_Update`: Calls `SndArpUpdate` after `SndEnvUpdate`.
    - `SndChUpdate`: Stores `BasePitch` index instead of calculating Period directly.
    - `SndArpUpdate`: Processes Arpeggio envelope, updates `ArpNoteOffset`, and recalculates Period (Base + Offset). Writes to hardware registers with optimization to avoid phase reset (checking High Byte change).
- **Frontend**:
    - Updated `static/js/audio.js` and `static/index.html` to add Arpeggio dropdown selector.
- **Verified**: `tests/audio_compilation_test.rs`, `tests/audio_priority_test.rs`, and all other integration tests passed with new memory layout.

### Phase 23: Audio - Envelope Editor UI (Completed)
- **Implemented**: Priority system for sound channels.
- **Backend**:
    - `AudioTrack` now has a `priority` field (u8).
    - `compile_audio_data` injects the priority byte into the track header (after instrument).
- **Codegen**:
    - `Sound_Init`: Clears extended sound RAM range ($0300-$031F) to include priority/instrument storage.
    - `Sound_Play`: Checks if `NewPriority >= CurrentPriority` (stored at `$0314` + ChannelIndex). If not, aborts. Updates priority if proceeding.
    - `SndChUpdate`: Clears priority (sets to 0) when a track finishes (Terminator).
- **Frontend**:
    - Added "Priority" input to `AudioTracker` UI.
- **Verified**: `tests/audio_priority_test.rs`.

### Miscellaneous Fixes
- **Assembler**: Fixed "Branch too far" error in `Sound_Update` by fixing duplicate labels (`SndPtrInc3` vs `SndPtrInc4`).
- **WaitVBlank**: Implemented `WAIT_VBLANK` command to allow safe PPU updates (like `Text.Print`) during the game loop.
- **Boolean Logic**: Fixed `Animation.finished` to set `$FF` (True) instead of `1`, ensuring `NOT` works correctly.
- **NMI Safety**: `TrampolineNMI` now saves CPU registers and Zero Page context (`$00`-$0F`), preventing 16-bit math corruption by interrupts.
- **String Heap**: Expanded to 16 slots (256 bytes) at `$0320`.
- **Signed Assignment Bug**: Fixed `Statement::Let` to correctly sign-extend `INT` values when assigning to `WORD` variables (using `STX` instead of zero-filling).

- **Next Steps**:
    - Start Phase 25a: Audio - SFX Engine Core.
    - Define `SoundEffect` struct and binary format.
    - Implement `SFX_Play` routine in Assembly.

### Memory Map
- **$0000-$00FF**: Zero Page.
    - `$E0`: Scroll X, `$E1`: Scroll Y.
    - `$E2`-$E3`: Random Seed (LFSR).
    - `$F8`: PPU Ctrl Shadow.
- **$0100-$01FF**: Stack.
- **$0200-$02FF**: OAM (Shadow Sprites).
- **$0300-$034F**: Sound Engine State.
    - `$0300`-$030F: Channel State.
    - `$0310`-$0313: Channel Instrument.
    - `$0314`-$0317: Channel Priority.
    - `$0318`-$0337: Vol/Pitch Envelope State.
    - `$0338`-$034B: Arpeggio State (ID, Pos, Timer, Offset, BasePitch).
    - `$034C`-$034F: LastPeriodH (for phase reset avoidance).
- **$0350-$044F**: String Heap.
- **$0450-$048F**: VBlank Buffer (Internal).
- **$0490-$07FF**: User Variables (DIM).
- **$8000-$CFFF**: PRG-ROM (Code).
- **$D000**: NTSC Period Table.
- **$D100**: Music Data.
- **$D480**: DPCM Sample Table (Addr, Len).
- **$D500**: Nametable Data.
- **$D900**: Envelope Data Table.
- **$E000**: Palette Data.
- **$E040**: DPCM Samples (Start).
- **$FF00**: Data Tables (Vectors pointers).
- **$FFFA**: Vectors (NMI, Reset, IRQ).

- **Pitfalls**:
    - `Text.Print` writes directly to the PPU ($2006/$2007). Use `WAIT_VBLANK` before calling this to avoid visual glitches.
    - `True` is `$FF`. Check assumptions in assembly injections if they rely on `1`.
    - **Audio Labels**: When injecting assembly strings in loops or multiple blocks, ensure labels are unique or use local labels if assembler supports it. `rs6502` global label reuse caused "Branch too far".
    - **DPCM Alignment**: Samples must start on 64-byte boundaries. Compiler handles padding.
    - **OAM Overflow**: `Sprite.Draw` drops sprites if 64 limit reached. Enable `Sprite.SetFlicker(1)` to mitigate limits via cycling.

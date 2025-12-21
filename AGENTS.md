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
Phase 1-29 are complete. Phase 30 is Next.

### Phase 29: Visual - World Editor (Completed)
- **Implemented**: `WorldLayout` struct in backend and `WorldEditor` in frontend.
- **Key Features**:
    - **World Struct**: Defines grid dimensions (`width`, `height`) and `data` (indices to Nametables).
    - **Editor UI**: Visual grid allowing placement of Nametables to create a larger world map.
    - **Persistence**: Saved to `assets.json` under `world`.

### Phase 28: Visual - Metatile Integration (Completed)
- **Implemented**: `Metatile` painting integrated into `MapEditor`.
- **Key Features**:
    - **Metatile Mode**: Allows painting 2x2 blocks on the nametable, updating tiles and attributes simultaneously.

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
- **$0380-$03BF**: VBlank Buffer.
- **$03C0-$04BF**: String Heap (256 bytes).
- **$04C0-$07FF**: User Variables (DIM).
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
    - `Text.Print` writes directly to the PPU ($2006/$2007). Use `WAIT_VBLANK` before calling this to avoid visual glitches.
    - `True` is `$FF`. Check assumptions in assembly injections if they rely on `1`.
    - **Audio Labels**: When injecting assembly strings in loops or multiple blocks, ensure labels are unique or use local labels if assembler supports it. `rs6502` global label reuse caused "Branch too far".
    - **DPCM Alignment**: Samples must start on 64-byte boundaries. Compiler handles padding.
    - **OAM Overflow**: `Sprite.Draw` drops sprites if 64 limit reached. Enable `Sprite.SetFlicker(1)` to mitigate limits via cycling.
    - **SFX Sequences**: `SequenceCanvas` modifies arrays in place.
    - **Frontend Validation**: When importing JSON, always validate fields exist to avoid `undefined` crashes in the editor.
    - **CHR Import**: Requires a 128x128 PNG for full bank import. Alpha channel is treated as color 0 (transparent). Nearest neighbor matching uses the *current* 4-color palette, not the full NES palette, so ensure the correct sub-palette is selected before importing.

- **Next Steps**:
    - Start Phase 30: Visual - Sprite Editor.
    - Create canvas for free placement of 8x8 sprite tiles or metatiles.
    - Implement Timeline UI for animation frames.
    - Export to `Metasprite` data format.

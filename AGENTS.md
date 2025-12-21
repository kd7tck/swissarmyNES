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
Phase 1-20 are complete. Phase 21 is Next.

### Phase 20: Random Number Generator (Completed)
- **Implemented**: `RND(max)` function and `RANDOMIZE seed` statement.
- **Details**:
    - **Runtime**:
        - Uses 16-bit LFSR at Zero Page `$E2`/`$E3`.
        - `RND(max)` returns `WORD` (0 to max-1).
        - `RANDOMIZE(seed)` initializes the LFSR. If seed is 0, defaults to `$FFFF` (non-zero required for LFSR).
    - **Codegen**:
        - Added `Runtime_Random` (LFSR logic).
        - Added `Runtime_Randomize`.
        - `Expression::Call("RND")` generates code to call `Runtime_Random` and then `Math_Mod16`.
    - **Verified**: `tests/rng_test.rs`.

### Miscellaneous Fixes
- **WaitVBlank**: Implemented `WAIT_VBLANK` command to allow safe PPU updates (like `Text.Print`) during the game loop.
- **Boolean Logic**: Fixed `Animation.finished` to set `$FF` (True) instead of `1`, ensuring `NOT` works correctly.
- **NMI Safety**: `TrampolineNMI` now saves CPU registers and Zero Page context (`$00`-$0F`), preventing 16-bit math corruption by interrupts.
- **Interrupts**: `INTERRUPT` declarations now compile to `RTS` to support the NMI/IRQ Trampoline wrapper which calls them via `JSR`.
- **String Heap**: Expanded to 16 slots (256 bytes) at `$0320`.
- **Safety**: Semantic Analyzer bans `RETURN` inside loops (`FOR`, `WHILE`, `DO`, `SELECT`) to prevent stack corruption.
- **Signed Assignment Bug**: Fixed `Statement::Let` to correctly sign-extend `INT` values when assigning to `WORD` variables (using `STX` instead of zero-filling).
- **RAM Overflow Check**: Added explicit check in `allocate_memory` to error if user variables exceed `$07FF`.

- **Next Steps**:
    - Start Phase 21: Audio - DPCM Support.
    - Implement DPCM sample import (WAV -> 1-bit Delta).
    - Audio Compiler support for DPCM.

### Memory Map
- **$0000-$00FF**: Zero Page.
    - `$E0`: Scroll X, `$E1`: Scroll Y.
    - `$E2`-$E3`: Random Seed (LFSR).
    - `$F8`: PPU Ctrl Shadow.
- **$0100-$01FF**: Stack.
- **$0200-$02FF**: OAM (Shadow Sprites).
- **$0300-$031F**: Sound Engine State.
- **$0320-$041F**: String Heap.
- **$0420-$045F**: VBlank Buffer (Internal).
- **$0460-$07FF**: User Variables (DIM).

- **Pitfalls**:
    - `Text.Print` writes directly to the PPU ($2006/$2007). Use `WAIT_VBLANK` before calling this to avoid visual glitches.
    - `True` is `$FF`. Check assumptions in assembly injections if they rely on `1`.
    - 16-bit Math helpers use ZP $06-$09 (now safe in NMI/IRQ due to context saving).
    - **OAM Overflow**: `Sprite.Draw` drops sprites if 64 limit reached. Enable `Sprite.SetFlicker(1)` to mitigate limits via cycling.
    - **Scrolling**: `Scroll.Set` updates shadow registers which are applied in the next NMI. Ensure `Scroll.Set` is called before VBlank if immediate effect is needed (though it applies next frame).

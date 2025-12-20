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
Phase 6-10 are complete.

### Phase 10: Macros (Completed)
- **Implemented**: `DEF MACRO` syntax, AST Preprocessor expansion.
- **Details**:
    - Macros are AST-based text replacements.
    - `Statement::Call` is replaced by the macro body with arguments substituted.
    - Support for nested macros and recursion limit (100).
    - Macros are defined at Top Level and removed before CodeGen.

### Phase 11: Standard Library - Controller (Completed)
- **Implemented**: `Controller` module (Read, IsPressed, IsHeld, IsReleased) and `Button` enum.
- **Details**:
    - **Parser**: Updated to allow keywords (like `Read`) as member names in dot notation.
    - **Runtime**: Uses ZP `$10`-`$13` for controller state.
    - **Analysis**: Pre-registers `Button` enum (A, B, Select, Start, Up, Down, Left, Right).

### Phase 13: Metasprite System (Completed)
- **Implemented**: `METASPRITE` definition syntax, `Sprite.Draw`, `Sprite.Clear` runtime helpers.
- **Details**:
    - **Syntax**: `METASPRITE name ... TILE x,y,t,a ... END METASPRITE`.
    - **Runtime**:
        - `Sprite.Draw(x, y, meta)`: Reads metasprite data and writes to OAM ($0200+). Uses linear allocation via $19 pointer.
        - `Sprite.Clear()`: Resets OAM pointer ($19) and clears OAM buffer.
    - **Codegen**:
        - Metasprite data is stored in `USER_DATA` segment.
        - Format: `Count`, then `X, Y, Tile, Attr` per tile.
        - Constants in TILE definition are resolved at compile time.

### Phase 14: Animation Engine (Completed)
- **Implemented**: `ANIMATION` definition syntax, `Animation.Play`, `Animation.Update`, `Animation.Draw`, `AnimState` struct.
- **Details**:
    - **Syntax**: `ANIMATION Name ... FRAME Metasprite, Duration ... [LOOP] END ANIMATION`.
    - **Runtime**:
        - `AnimState` struct: `ptr` (Word), `frame_index` (Byte), `timer` (Byte), `finished` (Byte).
        - `Animation.Play(state, anim)`: Initializes state.
        - `Animation.Update(state)`: Updates timer and frame index. Handles looping.
        - `Animation.Draw(x, y, state)`: Draws current frame's metasprite.
    - **Codegen**:
        - Animation data format: `Count`, `Loop`, then `FramePtr (Word)`, `Duration` per frame.
        - `rs6502` compatibility: Uses explicit unique labels for frame pointers to support `WORD` directive.

- **Next Steps**:
    - Phase 15: Object Pooling.

- **Pitfalls**:
    - `Text.Print` writes directly to the PPU ($2006/$2007). This is fast but must be done when rendering is disabled or during VBlank to avoid visual glitches.
    - `RETURN` inside a `CASE` block is unsafe because the stack is not cleaned up (it contains the Select value).
    - `True` is now `$FF` (was `1`). Check assumptions in assembly injections if they rely on `1`.
    - 16-bit Math helpers use ZP $06-$09.
    - **String Concatenation Limit**: The circular string heap has 4 slots.
    - **OAM Overflow**: `Sprite.Draw` stops filling if OAM wraps (64 sprites). It does not currently implement sprite cycling/flickering.

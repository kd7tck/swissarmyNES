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

### Phase Refinement: Arrays & Foundation & Strings
- **Implemented**:
    - **Logic & Math**: XOR operator, Unary Operators (`-`, `NOT`), 8-bit `AND`/`OR` fix, `ABS()`, `SGN()`, Bitwise Shifts (`<<`, `>>`).
    - **Memory**: Robust `PEEK` (Static optimization & Dynamic support), `POKE` (Dynamic fix).
    - **Arrays**: 1D Arrays, `DIM x(N)`.
    - **Strings**: `READ` support, `LEN()`, `ASC()`, `VAL()`, `CHR()`, `STR()`, `LEFT()`, `RIGHT()`, `MID()`.
    - **String Ops**: Concatenation (`+`), Comparison (`=`, `<>`).
    - **Control Flow**: `FOR` loops now support all numeric types (`Byte`, `Int`, `Word`) via synthesized AST expressions.
- **Details**:
    - **Boolean**: Standardized `True` to `$FF` (All ones) to support Bitwise `NOT` correctly.
    - **Codegen**:
        - Implemented `BinaryOperator::ShiftLeft/ShiftRight` (8/16-bit).
        - Implemented `Runtime_StringConcat` and `Runtime_StringCompare`.
        - `BinaryOperator::Add` handles String concatenation.
        - `BinaryOperator::Equal/NotEqual` handles String comparison.
        - **String Heap**: Circular buffer (4x16 bytes) at `$02A0` for dynamic string results.
    - **Parser/AST**: Added `ShiftLeft`/`ShiftRight` tokens and precedence.

- **Next Steps**:
    - Phase 11: Standard Library - Controller.

- **Pitfalls**:
    - `RETURN` inside a `CASE` block is unsafe because the stack is not cleaned up (it contains the Select value).
    - `True` is now `$FF` (was `1`). Check assumptions in assembly injections if they rely on `1`.
    - 16-bit Math helpers use ZP $06-$09.
    - **String Concatenation Limit**: The circular string heap has 4 slots. Complex expressions generating more than 4 simultaneous temporary strings (e.g. `s = a + b + c + d + e`) may overwrite early slots, potentially corrupting data if not evaluated strictly. Simple concatenations are safe.

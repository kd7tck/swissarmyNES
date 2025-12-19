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
Phase 6 (Advanced Math), Phase 7 (Switch/Case), and Phase 8 (Structs) are complete.

### Phase 6: Advanced Math
- **Implemented**: `INT` type (8-bit signed), 16-bit Multiplication/Division (`Math_Mul16`, `Math_Div16`), and Signed Comparisons.
- **Details**:
    - `src/compiler/codegen.rs`: `BinaryOp` now supports 16-bit Multiply/Divide via helper routines.
    - Comparisons (`<`, `>`, `<=`, `>=`) handle Signed logic (checking Overflow flag) if `INT` is involved.
    - Integer Literals are promoted to `DataType::Word` (if positive) or `DataType::Int` (if negative) to ensure 16-bit math precision by default.
    - `Math_Mul16` and `Math_Div16` operate on A/X and $00/$01, using ZP $06-$09 as scratchpad.

### Phase 7: Switch/Case
- **Implemented**: `SELECT CASE` statement.
- **Details**:
    - `src/compiler/parser.rs`: Added `parse_select`.
    - `src/compiler/codegen.rs`: Implemented `Statement::Select`. Pushes `Expression` to Stack, then peeks it for each `CASE` comparison.
    - 16-bit comparisons save `X` to `$01` before using `TSX` to preserve the high byte.

### Phase 8: Structs
- **Implemented**: `TYPE` definitions and dot-notation access.
- **Details**:
    - **AST**: `Statement::Let` now takes `Expression` as target to support `MemberAccess` (L-values). `Expression::MemberAccess` added.
    - **Parser**: Parses `TYPE ... END TYPE` and `expr.member`. Dot has high precedence (Call).
    - **Codegen**: Allocates memory based on sum of member sizes. Resolves member addresses statically relative to base variable.
    - **Limitations**: Structs cannot be used in arithmetic or assigned directly (must assign members). Arrays of structs not yet implemented.

- **Next Steps**:
    - Phase 9: Enums & Constants.

- **Pitfalls**:
    - `RETURN` inside a `CASE` block is unsafe because the stack is not cleaned up (it contains the Select value). Use `GOTO` out of the block if early exit is needed, or structure code to fall through.
    - Integer literals returning `Word` means `Byte + Literal` promotes to 16-bit addition. This is safer for overflow but slower.
    - `Math_Div16` is currently Unsigned. Signed division logic is not fully implemented (treated as Unsigned for now).
    - 16-bit Math helpers use ZP $06-$09. Ensure these don't conflict with future Interrupt usage or other scratchpads.
    - `Statement::Let` change required updating all tests that manually constructed ASTs. Future AST changes should be mindful of this.

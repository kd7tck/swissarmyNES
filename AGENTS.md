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
- **Implemented**: `INT` type (8-bit signed), 16-bit Multiplication/Division (`Math_Mul16`, `Math_Div16`), Signed Division (`Math_Div16_Signed`), Modulo (`MOD`), and Signed Comparisons.
- **Details**:
    - `src/compiler/codegen.rs`: `BinaryOp` now supports 16-bit Multiply/Divide/Modulo via helper routines.
    - Comparisons (`<`, `>`, `<=`, `>=`) handle Signed logic (checking Overflow flag) if `INT` is involved.
    - Integer Literals are promoted to `DataType::Word` (if positive) or `DataType::Int` (if negative) to ensure 16-bit math precision by default.
    - `Math_Mul16` and `Math_Div16` operate on A/X and $00/$01, using ZP $06-$09 as scratchpad.
    - `Math_Div16_Signed` handles signed division and modulo.
    - `MOD` operator added to AST, Lexer, Parser.

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

### Phase Refinement: Arrays & Foundation & Strings
- **Implemented**:
    - **Logic & Math**: XOR operator, Unary Operators (`-`, `NOT`), 8-bit `AND`/`OR` fix, `ABS()`, `SGN()`.
    - **Memory**: Robust `PEEK` (Static optimization & Dynamic support), `POKE` (Dynamic fix).
    - **Arrays**: 1D Arrays, `DIM x(N)`.
    - **Strings**: `READ` support, `LEN()`.
- **Details**:
    - **Boolean**: Standardized `True` to `$FF` (All ones) to support Bitwise `NOT` correctly.
    - **Codegen**:
        - Implemented `BinaryOperator::Xor` (8/16-bit).
        - Fixed `BinaryOperator` for 8-bit `And`/`Or`/`Divide` (was missing).
        - Implemented `Expression::UnaryOp` (`Negate`, `Not`).
        - Implemented `Expression::Peek` with constant address optimization.
        - Updated `generate_address_expression` to handle dynamic address expressions (e.g., `POKE(base + 1, val)`).
        - Added built-in `ABS` and `SGN` logic in `Expression::Call`.
    - **Parser/AST**: Added `Xor` token and precedence.
    - **Analysis**: Added `ABS`, `SGN` to built-in allowlist.

- **Next Steps**:
    - Phase 9: Enums & Constants.
    - More String manipulation functions (`LEFT`, `RIGHT`, `MID` - requires String Heap strategy).

- **Pitfalls**:
    - `RETURN` inside a `CASE` block is unsafe because the stack is not cleaned up (it contains the Select value).
    - `True` is now `$FF` (was `1`). Check assumptions in assembly injections if they rely on `1`.
    - 16-bit Math helpers use ZP $06-$09.

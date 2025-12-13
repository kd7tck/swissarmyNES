# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status
**Phase 11: End-to-End "Hello World"** - The compiler can now compile basic variable assignments and constants into a valid NES ROM binary.

*   **Recent Milestones:**
    *   **Architecture:** Split into a library and binary structure to support robust integration testing.
    *   **Assembler:** Integrated `rs6502` to assemble generated code into machine language.
    *   **Compiler:** Added support for `LET` statements and integer `CONST` resolution.
    *   **Testing:** Implemented end-to-end testing pipeline (`tests/e2e_test.rs`) that verifies compilation from source to binary opcodes.
    *   **CI/CD:** Enforced strict formatting checks in GitHub Actions.

## Features
- **SwissBASIC**: A hybrid language combining BASIC syntax with inline 6502 Assembly.
  - *New:* `LET` keyword for variable assignment.
  - *New:* `CONST` definitions for integer values.
- **Zero-Friction Toolchain**: Cloud-based compilation.
- **Visual Editors**: Sprite, Map, and Palette editors (Planned).

## Getting Started

### Prerequisites
- Rust (latest stable)
- Cargo

### Running Locally

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd SwissArmyNES
   ```

2. **Run the server:**
   ```bash
   cargo run
   ```
   The server will start at `http://0.0.0.0:3000`.

3. **Run Tests:**
   The project now includes unit tests and integration tests.
   ```bash
   cargo test
   ```

### Project Structure
*   `src/lib.rs`: Core compiler and library logic.
*   `src/main.rs`: Axum web server entry point.
*   `src/compiler/`: Compiler modules (Lexer, Parser, AST, Analysis, Codegen, Assembler).
*   `tests/`: Integration tests (Server, End-to-End Compiler).

## License
[License Information]

# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status
**Phase 12: Web Code Editor** - The frontend now includes a functional code editor with line numbers and basic syntax highlighting.

*   **Recent Milestones:**
    *   **Phase 11: End-to-End "Hello World"**: Verified that the compiler generates valid NES ROMs capable of changing background colors (PPU writes).
    *   **Phase 12: Web Editor**: Implemented a lightweight, dependency-free syntax highlighter for SwissBASIC in the web interface.
    *   **Compiler**: Added support for `LET` statements, `CONST` definitions, and basic `POKE` operations.
    *   **Testing**: Robust integration tests verify the full compilation pipeline from source to binary.

## Features
- **SwissBASIC**: A hybrid language combining BASIC syntax with inline 6502 Assembly.
  - `LET` keyword for variable assignment.
  - `CONST` definitions for integer values.
  - `POKE` for direct memory access.
- **Web IDE**:
  - **Code Editor**: Syntax highlighting, line numbers, and basic auto-indentation.
  - **Zero-Friction Toolchain**: Cloud-based compilation (Backend integration in progress).
- **Planned**: Visual Editors for Sprites, Maps, and Palettes.

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
   Open your browser to see the Web IDE.

3. **Run Tests:**
   The project includes unit tests and end-to-end integration tests.
   ```bash
   cargo test
   ```

### Project Structure
*   `src/lib.rs`: Core compiler and library logic.
*   `src/main.rs`: Axum web server entry point.
*   `src/compiler/`: Compiler modules (Lexer, Parser, AST, Analysis, Codegen, Assembler).
*   `static/`: Frontend assets (HTML, CSS, JS).
*   `tests/`: Integration tests.

## License
[License Information]

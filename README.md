# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status
**Phase 4: The AST Definition** - The compiler's Abstract Syntax Tree (AST) has been defined. This includes data structures for Expressions, Statements, Functions, and Loops, setting the stage for the parser implementation.
*   **Previous Phases Completed:**
    *   **Phase 1: Project Initialization:** Rust backend set up with Axum/Tokio.
    *   **Phase 2: Frontend Foundation:** Basic SPA structure with navigation and vector-based CSS.
    *   **Phase 3: The Lexer:** The compiler's lexical analyzer (Lexer) has been implemented to tokenize SwissBASIC source code.

## Features (Planned)
- **SwissBASIC**: A hybrid language combining BASIC syntax with inline 6502 Assembly.
- **Zero-Friction Toolchain**: Cloud-based compilation (or WASM).
- **Visual Editors**: Sprite, Map, and Palette editors.

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
   To verify the compiler components (Lexer and AST), run:
   ```bash
   cargo test
   ```

## License
[License Information]

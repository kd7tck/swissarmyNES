# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status
**Phase 7: Code Generation - Variables** - The compiler is now entering the code generation phase.
*   **Previous Phases Completed:**
    *   **Phase 1: Project Initialization:** Rust backend set up with Axum/Tokio.
    *   **Phase 2: Frontend Foundation:** Basic SPA structure with navigation and vector-based CSS.
    *   **Phase 3: The Lexer:** The compiler's lexical analyzer (Lexer) has been implemented to tokenize SwissBASIC source code.
    *   **Phase 4: The AST Definition:** The compiler's Abstract Syntax Tree (AST) has been defined.
    *   **Phase 5: The Parser:** Recursive descent parser implemented to generate AST from tokens.
    *   **Phase 6: Symbol Table & Scope Analysis:** Symbol table implemented for variable tracking and scope management.

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
   To verify the compiler components (Lexer, Parser, AST, Analysis), run:
   ```bash
   cargo test
   ```

## License
[License Information]

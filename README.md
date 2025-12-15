# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status
**Phase 14: Project Management System** - A project management system has been implemented to Create, Save, and Load projects directly from the IDE.

*   **Recent Milestones:**
    *   **Phase 19: Nametable (Map) Editor**: Implemented a visual editor to paint tiles onto a 32x30 grid.
    *   **Phase 18: CHR Editor Tools**: Added Flood Fill, Shift, Flip tools, and Bank View to the Tile Editor.
    *   **Phase 17: CHR (Tile) Editor**: Implemented a basic 8x8 Tile Editor with drawing capabilities and bank navigation.
    *   **Phase 16: Palette Editor**: Implemented a UI for selecting NES system colors and assigning them to sub-palettes.
    *   **Phase 15: Asset Management**: Defined JSON schemas for CHR banks, Palettes, and Nametables.
    *   **Phase 14: Project Management**: Users can now create named projects, which are persisted to the local filesystem.
    *   **Phase 13: Compiler API**: Implemented `/api/compile` endpoint and connected it to the "Compile" button in the frontend.
    *   **Phase 12: Web Editor**: Implemented a lightweight, dependency-free syntax highlighter for SwissBASIC in the web interface.
    *   **Phase 11: End-to-End "Hello World"**: Verified that the compiler generates valid NES ROMs capable of changing background colors (PPU writes).
    *   **Phase 8-10: Compiler Core**: Implemented control flow (`IF`, `WHILE`, `FOR`), variables, expressions, and inline assembly support.

## Features
- **Project Management**:
  - **Create**: Create new projects with unique names.
  - **Load/Save**: Persist your source code across sessions.
  - **Explorer**: A sidebar "Project Explorer" to easily switch between projects.
  - **Storage**: Projects are stored as folders in the `projects/` directory in the application root.
- **SwissBASIC**: A hybrid language combining BASIC syntax with inline 6502 Assembly.
  - **Variables**: `DIM` (BYTE, WORD), `LET` assignments.
  - **Control Flow**: `IF`, `WHILE`, `FOR...NEXT`, `DO...LOOP`.
  - **Hardware Access**:
    - `POKE(addr, val)`: Supports constant addresses (16-bit) and dynamic Zero-Page addresses (8-bit).
    - `PEEK(addr)`: Supports constant addresses.
    - Inline `ASM` blocks.
  - **Structure**: `SUB`, `INTERRUPT` definitions.
- **Web IDE**:
  - **Code Editor**: Syntax highlighting, line numbers, and basic auto-indentation.
  - **Instant Compilation**: Click "Compile" to generate and download a `.nes` ROM file immediately.
- **Visual Editors**:
  - **Palette Editor**: Manage system colors and sub-palettes.
  - **Tile Editor**: Edit 8x8 CHR tiles with real-time feedback.
  - **Map Editor**: Paint 8x8 tiles onto a 32x30 nametable grid (screen).

## Limitations
- **8-Bit Math**: Mathematical expressions are currently evaluated using 8-bit arithmetic (Accumulator). Complex 16-bit math is not yet fully supported.
- **Dynamic Addressing**: `POKE` with a variable address is currently limited to addresses in the Zero Page ($00-$FF). Use `CONST` for hardware registers (e.g. `$2006`).

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
*   `projects/`: Directory where user projects are stored (created at runtime).
*   `src/lib.rs`: Core compiler and library logic.
*   `src/main.rs`: Axum web server entry point.
*   `src/server/`: Server modules including API handlers (`api.rs`) and project logic (`project.rs`).
*   `src/compiler/`: Compiler modules (Lexer, Parser, AST, Analysis, Codegen, Assembler).
*   `static/`: Frontend assets (HTML, CSS, JS).
*   `tests/`: Integration tests.

## Troubleshooting

### CI Failure: Clippy Warnings
The CI pipeline is configured to treat warnings as errors. To avoid build failures due to false positives regarding "dead code" (unused functions/variables), the workflow is configured to allow dead code warnings:
`cargo clippy -- -D warnings -A dead_code`

If you encounter other Clippy failures, please address them by fixing the code or suppressing specific warnings if necessary.

## License
[License Information]

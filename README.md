# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status
**Phase 14: Project Management System** - A project management system has been implemented to Create, Save, and Load projects directly from the IDE.

*   **Recent Milestones:**
    *   **Phase 14: Project Management**: Users can now create named projects, which are persisted to the local filesystem.
    *   **Phase 13: Compiler API**: Implemented `/api/compile` endpoint and connected it to the "Compile" button in the frontend.
    *   **Phase 12: Web Editor**: Implemented a lightweight, dependency-free syntax highlighter for SwissBASIC in the web interface.
    *   **Phase 11: End-to-End "Hello World"**: Verified that the compiler generates valid NES ROMs capable of changing background colors (PPU writes).

## Features
- **Project Management**:
  - **Create**: Create new projects with unique names.
  - **Load/Save**: Persist your source code across sessions.
  - **Explorer**: A sidebar "Project Explorer" to easily switch between projects.
  - **Storage**: Projects are stored as folders in the `projects/` directory in the application root.
- **SwissBASIC**: A hybrid language combining BASIC syntax with inline 6502 Assembly.
  - `LET` keyword for variable assignment.
  - `CONST` definitions for integer values.
  - `POKE` for direct memory access.
- **Web IDE**:
  - **Code Editor**: Syntax highlighting, line numbers, and basic auto-indentation.
  - **Instant Compilation**: Click "Compile" to generate and download a `.nes` ROM file immediately.
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

# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a robust Rust backend compiler with a lightweight, vector-based HTML/JS frontend to provide a modern, zero-setup workflow for retro game development.

## Current Status
The project is currently in active development, following a strict roadmap defined in `DESIGN.md`.
**Completed Phases:** 1 through 27.
- **Core Compiler**: Lexer, Parser, AST, Symbol Table, Code Generation, Assembly.
- **Language Features**: Strings, Arrays (`DIM`), Structures (`TYPE`), Enums (`ENUM`), Macros (`DEF MACRO`), Advanced Math (16-bit, Signed), Control Flow (`SELECT CASE`, `FOR`, `WHILE`).
- **Standard Library**: Controller Input (`Controller.Read`, `IsPressed`).
- **Tools**: Palette Editor, Tile (CHR) Editor, Map (Nametable) Editor, Metatile Editor, Audio Tracker.

## Features

### SwissBASIC Language
A hybrid language designed for the NES, combining BASIC simplicity with low-level control.
- **Data Types**: `BYTE` (unsigned 8-bit), `INT` (signed 8-bit), `WORD` (unsigned 16-bit), `STRING` (dynamic text).
- **Structures**:
  - `TYPE`: Define custom data structures (e.g., `TYPE Player \n x AS BYTE \n y AS BYTE \n END TYPE`).
  - `ENUM`: Define enumerated constants.
  - `DIM`: 1D Arrays (e.g., `DIM buffer(10) AS BYTE`).
- **Control Flow**:
  - `IF ... THEN ... ELSEIF ... END IF`
  - `SELECT CASE ... CASE ... END SELECT` (supports ranges `TO` and comparisons `IS`).
  - `FOR ... NEXT` (supports variable steps).
  - `WHILE ... WEND` / `DO ... LOOP`.
- **Math & Logic**:
  - Full 16-bit arithmetic (`+`, `-`, `*`, `/`, `MOD`).
  - Bitwise operations (`AND`, `OR`, `XOR`, `NOT`, `<<`, `>>`).
  - Built-in functions: `ABS`, `SGN`, `LEN`, `ASC`, `VAL`, `CHR`, `STR`.
- **Hardware Access**:
  - `PEEK` / `POKE` for direct memory access.
  - Inline `ASM` blocks for critical assembly code.
  - `INTERRUPT` handlers (NMI, IRQ) and dynamic vector mapping (`ON NMI DO ...`).
- **Macros**: Preprocessor macros via `DEF MACRO` for code reuse.
- **Multi-File Support**: `INCLUDE "file.swiss"` to organize projects.

### Integrated Tools
- **Project Management**: Create, Load, and Save projects locally.
- **Code Editor**: Syntax highlighting, line numbers, and error reporting.
- **Visual Editors**:
  - **Palette**: Edit system colors and sub-palettes.
  - **Tile (CHR)**: Draw 8x8 sprites and tiles with real-time feedback.
  - **Map**: Paint tiles onto a 32x30 nametable grid.
  - **Metatile**: Create 16x16 reusable tile blocks with attributes.
- **Audio Tracker**:
  - Compose music and SFX for Pulse 1, Pulse 2, and Triangle channels.
  - Custom envelopes for Volume and Duty Cycle.
  - `PLAY_SFX` command integration.

### Compiler & Runtime
- **Instant Compilation**: Generates a valid `.nes` ROM file in milliseconds.
- **Optimized Runtime**: Custom assembly routines for math, string handling, and audio mixing.
- **Memory Management**: Automatic allocation of Zero Page and RAM variables.

## Getting Started

### Prerequisites
- **Rust**: Latest stable version.
- **Cargo**: Included with Rust.

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
   The server will start at `http://0.0.0.0:3000`. Open your browser to access the IDE.

3. **Run Tests:**
   The project includes a comprehensive suite of unit and integration tests.
   ```bash
   cargo test
   ```

## Project Structure

### Backend (`src/`)
- **`lib.rs`**: Library entry point exposing compiler and server logic.
- **`main.rs`**: Application entry point, sets up the Axum server.
- **`server/`**: API handlers and file system logic.
- **`compiler/`**: The heart of SwissArmyNES.
  - `lexer.rs` / `parser.rs` / `ast.rs`: Language frontend.
  - `analysis.rs`: Semantic analysis and type checking.
  - `codegen.rs`: Generates 6502 assembly from AST.
  - `assembler.rs`: Assembles generated code into NES ROM binary.
  - `audio.rs`: Compiles tracker data into sound engine bytecode.

### Frontend (`static/`)
- **`index.html`**: Main single-page application entry.
- **`js/`**: Vanilla JavaScript modules.
  - `app.js`: Router and main logic.
  - `editor.js`: Code editor implementation.
  - `chr.js`, `map.js`, `palette.js`: Visual editors.
  - `audio.js`: Audio tracker UI.

## Documentation
- **`DESIGN.md`**: The master roadmap and technical specification.
- **`AGENTS.md`**: Guidelines for AI contributors, including coding standards and context.

## Contributing
This project heavily utilizes AI agents for development. Contributors should:
1.  Read `AGENTS.md` thoroughly.
2.  Follow the "Test Constantly" directive.
3.  Ensure all code passes `cargo clippy` and `cargo fmt`.

## License
[License Information]

# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status
**Phase 24: Audio Compilation Complete** - The Audio pipeline is fully implemented, including a Tracker UI, data compilation, and binary injection into the ROM.

*   **Recent Milestones:**
    *   **Phase 24: Audio Compilation**: Implemented audio data compilation and injection into the ROM.
    *   **Phase 23: Audio Tracker UI**: Added a piano-roll interface for composing music and sound effects.
    *   **Phase 22: Audio Engine**: Implemented a basic 6502 sound engine supporting PlaySfx commands.
    *   **Phase 21: Asset Compilation**: Added support for compiling and injecting Nametables (Maps) into the ROM startup.
    *   **Phase 20: Map Attributes**: Added support for painting attribute tables (color palettes) on the Map Editor.
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
    - **Note**: `FOR` loops support both positive and negative `STEP` values (constants).
  - **Math**:
    - Supports `+`, `-`, `*` (multiply), `/` (divide), `AND`, `OR`.
    - **Note**: Expressions are evaluated as 8-bit unsigned integers (0-255). Division truncates.
  - **Hardware Access**:
    - `POKE(addr, val)`: Supports constant addresses (16-bit) and dynamic addresses (16-bit via WORD variables).
    - `PEEK(addr)`: Supports constant addresses and dynamic addresses (via WORD variables).
    - Inline `ASM` blocks.
  - **Audio**:
    - `PLAY_SFX(id)`: Play a sound effect. `1` = Jump, `2` = Shoot.
    - **Tracker**: Built-in audio tracker for composing sequences (Pulse 1, Pulse 2, Triangle).
  - **Structure**:
    - `SUB`: Define subroutines.
    - `INTERRUPT`: Define interrupt handlers (e.g., `INTERRUPT NMI() ... END INTERRUPT`).
    - `ON <Vector> DO <Routine>`: Map interrupts to routines at runtime (e.g., `ON NMI DO MyVBlank`).
- **Web IDE**:
  - **Code Editor**: Syntax highlighting, line numbers, and basic auto-indentation.
  - **Instant Compilation**: Click "Compile" to generate and download a `.nes` ROM file immediately.
- **Visual Editors**:
  - **Palette Editor**: Manage system colors and sub-palettes.
  - **Tile Editor**: Edit 8x8 CHR tiles with real-time feedback.
  - **Map Editor**: Paint 8x8 tiles onto a 32x30 nametable grid (screen) and assign color attributes (palettes).

## Limitations
- **8-Bit Math**: Mathematical expressions are evaluated using 8-bit arithmetic (0-255). Overflow wraps around. 16-bit math is limited to simple assignments/copies to `WORD` variables.
- **FOR Loops**: Loop counters and steps are strictly 8-bit (0-255). Using a `WORD` variable as a counter will only check its low byte.
- **Dynamic Addressing**: Use `WORD` variables for `POKE` and `PEEK` to access dynamic addresses. Use `CONST` for hardware registers (e.g. `$2006`).
- **RAM Limit**: Global variables are allocated starting at `$0300`. The compiler will error if variable allocation exceeds the NES RAM limit ($07FF).
- **Emulator**: Phase 25 (Emulator Integration) is currently pending. Users must download the ROM and run it in an external emulator (e.g. FCEUX, Mesen).

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

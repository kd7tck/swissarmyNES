# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status


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
    - **Instruments**: Select from various Duty Cycles (12.5%, 25%, 50%, 75%) and Envelopes (Constant, Decay).
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
- **Dynamic Addressing**: Use `WORD` variables for `POKE` and `PEEK` to access dynamic addresses. Use `CONST` for hardware registers (e.g. `$2006`).
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

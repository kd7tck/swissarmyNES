# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a robust Rust backend compiler with a lightweight, vector-based HTML/JS frontend to provide a modern, zero-setup workflow for retro game development.

## Current Status
The project is currently in active development, following a strict roadmap defined in `DESIGN.md`.
For the current completion plan, read the [developer handoff through phase 40](docs/DEVELOPER_HANDOFF_THROUGH_PHASE_40.md), [baseline audit](docs/BASELINE_AUDIT.md), and [acceptance worksheet](docs/PHASE_40_ACCEPTANCE_CHECKLIST.md). The September 2026 audit found that native tests pass but the production compile API fails on minimal programs; historical completion labels below do not constitute end-to-end acceptance.
**Historically marked complete:** phases 1 through 30.
- **Core Compiler**: Lexer, Parser, AST, Symbol Table, Code Generation, Assembly.
- **Language Features**: Strings, Arrays (`DIM`), Structures (`TYPE`), Enums (`ENUM`), Macros (`DEF MACRO`), Advanced Math (16-bit, Signed), Control Flow (`SELECT CASE`, `FOR`, `WHILE`).
- **Standard Library**: Controller Input (`Controller.Read`, `IsPressed`), Sprite/Animation System, Object Pooling.
- **Tools**: Palette Editor, Tile (CHR) Editor, Map (Nametable) Editor, Metatile Editor, World Editor, Sprite/Animation Editor, Audio Tracker.

## Features

### SwissBASIC Language
A hybrid language designed for the NES, combining BASIC simplicity with low-level control.
- **Data Types**: `BYTE` (unsigned 8-bit), `INT` (signed 8-bit), `WORD` (unsigned 16-bit), `STRING` (dynamic text).
- **Structures**:
  - `TYPE`: Define custom data structures (e.g., `TYPE Player \n x AS BYTE \n y AS BYTE \n END TYPE`).
  - `ENUM`: Define enumerated constants.
  - `DIM`: 1D Arrays (e.g., `DIM buffer(10) AS BYTE`).
  - `METASPRITE`: Define composite sprites from multiple 8x8 tiles.
  - `ANIMATION`: Define animation sequences for metasprites.
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
  - **World**: Arrange maps (Nametables) into a larger game world grid.
  - **Sprite**: Design composed characters (Metasprites) and define Animations.
- **Audio Tracker**:
  - Compose music and SFX for Pulse 1, Pulse 2, Triangle, and DMC channels.
  - Custom envelopes for Volume, Pitch, and Duty Cycle.
  - `PLAY_SFX` command integration.

### Compiler & Runtime
- **Instant Compilation**: Generates a valid `.nes` ROM file in milliseconds.
- **Optimized Runtime**: Custom assembly routines for math, string handling, and audio mixing.
- **Memory Management**: Automatic allocation of Zero Page and RAM variables.

## Getting Started

### Prerequisites
- **Rust**: 1.98.0, including the `wasm32-unknown-unknown` target.
- **Cargo**: Included with Rust.
- **Node.js**: 22 or newer, for fixture downloads and frontend/WASM tests.
- **Git and tar**: Used to prepare the checksum-pinned emulator dependency.

### Running Locally

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd SwissArmyNES
   ```

2. **Build the browser emulator from source:**
   Generated WASM and bindings are deliberately excluded from version control.
   ```bash
   node scripts/prepare-core.mjs
   rustup toolchain install 1.98.0 --profile minimal --component rustfmt --component clippy --target wasm32-unknown-unknown
   cargo +1.98.0 install wasm-bindgen-cli --version 0.2.106 --locked
   cargo +1.98.0 build -p swiss-emulator --release --target wasm32-unknown-unknown --locked
   wasm-bindgen --target web --out-dir static/wasm --out-name swiss_emulator target/wasm32-unknown-unknown/release/swiss_emulator.wasm
   ```
   Repeat the last two commands after changing emulator source. Keep the binding
   generator version aligned with `Cargo.lock`.
   The preparation script verifies TetaNES 0.12.2 against its registry checksum,
   applies `patches/tetanes-core-0.12.2.patch`, and writes the patched dependency
   into ignored `.tools/tetanes-core`. Run it before any Cargo command on a fresh
   checkout and again after changing the patch. It uses Cargo's archive cache
   when available; otherwise it downloads the pinned package. Run preparation
   while no build is using that generated dependency directory.

3. **Run the server:**
   ```bash
   cargo run
   ```
   The server will start at `http://0.0.0.0:3000`. Open your browser to access the IDE.

4. **Run Tests:**
   The project includes a comprehensive suite of unit and integration tests.
   ```bash
   node scripts/fetch-test-fixtures.mjs
   cargo +1.98.0 test --workspace --all-targets --locked
   node --test tests/js/*.test.cjs
   ```
   The fixture script downloads the external nestest ROM and reference trace at
   a pinned revision and verifies SHA-256 before use. Subsequent runs verify local
   files without downloading again. These inputs are ignored, as are generated
   browser artifacts. WASM tests require the build in step 2.

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
  - `metatile.js`, `world.js`, `sprite.js`: Advanced visual editors.

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

CPU validation now includes the complete 8,991-record canonical nestest trace and exhaustive PLP/PHP/RTI status serialization checks. See docs/IMPLEMENTATION_RESUME.md for evidence and remaining phase gates. This does not constitute phase-40 acceptance.


The generated browser WASM also matches all 8,991 canonical CPU records. Run node --test tests/js/*.test.cjs for bundle, palette, and regional pacing regressions. Playback cadence now follows the backend NTSC/PAL/Dendy region.

ON NMI DO Handler and ON IRQ DO Handler accept zero-argument routines, including routines in switchable banks. Binding updates are atomic and handlers may rebind themselves. The compiler supports up to 255 distinct dynamically bound handlers, subject to available ROM space.

Runtime interrupt regressions cover helper argument/audio scratch preservation and inactive-channel updates. The measured idle NMI takes 1774 CPU cycles including DMA; active audio, VRAM and user-handler timing still needs validation.

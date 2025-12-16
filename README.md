# SwissArmyNES

A web-based IDE and Compiler for NES game development.

## Project Overview

**SwissArmyNES** is a comprehensive tool for creating Nintendo Entertainment System (NES) games. It combines a modern web-based interface with a custom hybrid language ("SwissBASIC") that bridges the gap between high-level ease of use and low-level hardware control.

The system features:
*   **SwissBASIC Compiler:** Compiles BASIC-like syntax with inline Assembly directly to NES ROMs.
*   **Visual Editors:** Integrated tools for Sprite (CHR), Map (Nametable), Palette, and Audio editing.
*   **Project Management:** Save and load projects with all assets and source code.
*   **Web-Based:** Runs in the browser (frontend) with a Rust backend.

## Current Status

**Completed Phases (1-24):**
*   **Compiler Core:** Lexer, Parser, Symbol Table, Semantic Analysis, Code Generation.
*   **Language Features:**
    *   Variables (`DIM`, `LET`), Constants (`CONST`).
    *   Control Flow (`IF`, `WHILE`, `FOR`, `DO...LOOP`).
    *   Subroutines (`SUB`, `CALL`, `RETURN`).
    *   Interrupt Handlers (`INTERRUPT`, `ON NMI/IRQ DO`).
    *   Inline Assembly (`ASM ... END ASM`).
    *   Memory Access (`PEEK`, `POKE`).
    *   Data Types (`BYTE`, `WORD`, `BOOL`).
*   **Assembler Integration:** Generates valid iNES ROMs using `rs6502` assembler backend.
*   **Frontend Editors:**
    *   Code Editor with syntax highlighting.
    *   Palette Editor (NES system colors).
    *   CHR (Tile) Editor.
    *   Map (Nametable) Editor.
    *   Audio Tracker (3-channel sequencer).
*   **Asset Pipeline:**
    *   Compiles graphics (CHR, Palettes, Nametables) and audio data directly into the ROM.
    *   Automatic handling of NTSC Period Tables and Sound Engine injection.

**In Progress:**
*   **Phase 25:** Emulator Integration (WASM).
*   **Phase 26-27:** Debugging Tools (CPU/Memory View).
*   **Phase 28-30:** Optimization, Polish, and Documentation.

## Getting Started

### Prerequisites
*   Rust (latest stable)
*   Cargo

### Running the Server
1.  Clone the repository.
2.  Run `cargo run` in the root directory.
3.  Open `http://localhost:3000` in your web browser.

### Creating a Project
1.  Click "New Project" in the sidebar.
2.  Enter a name (e.g., "MyGame").
3.  Start coding in the Code Editor or drawing in the Graphics Editor.
4.  Click "Compile" to generate and download a `.nes` ROM file.
5.  Run the downloaded ROM in an NES emulator (e.g., FCEUX, Mesen).

## Troubleshooting

*   **Compilation Errors:** Check the server logs or the alert message in the browser. Common issues include syntax errors or undefined variables.
*   **Audio:** Ensure your browser allows autoplay if testing audio features (once emulator is integrated).
*   **Linting:** The project enforces `cargo clippy -- -D warnings`. Ensure your code is clean before submitting.

## Development

To run tests:
```bash
cargo test
```

To run clippy:
```bash
cargo clippy -- -D warnings
```

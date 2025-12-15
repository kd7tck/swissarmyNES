# SwissArmyNES Design Document

## 1. Project Overview
**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It targets hobbyists and enthusiasts by offering a streamlined workflow that bridges the gap between high-level ease of use and low-level hardware control.

The system consists of a robust **Rust** backend acting as the compiler and asset manager, and a lightweight, beautiful **HTML/JS** frontend that serves as the visual workspace.

## 2. Core Philosophy
1.  **Hybrid Language:** A new language ("SwissBASIC") that combines the readability of BASIC with the raw power of inline 6502 Assembly.
2.  **Zero-Friction Toolchain:** No local toolchain installation required; the compiler runs on the server (or via WASM), delivering instant ROM builds.
3.  **Visual First:** Integrated editors for sprites, maps, and palettes that feel modern and use vector-based UI elements for crisp scaling.
4.  **Granular Evolution:** A development roadmap split into 30+ distinct phases to ensure steady, manageable progress.

## 3. Technology Stack

### Backend (Server & Compiler)
*   **Language:** Rust
*   **Web Framework:** Axum or Actix-web (for high-performance HTTP handling).
*   **Role:**
    *   Serve static frontend assets.
    *   Handle project persistence (saving/loading source files and assets).
    *   (Optional) Server-side compilation services (though the compiler core may be designed to compile to WASM for client-side execution).

### Frontend (Client)
*   **Language:** Vanilla JavaScript / TypeScript (No heavy frameworks like React/Angular to keep it "minimal").
*   **Styling:** CSS3 with a focus on Vector Graphics (SVG) for UI icons and layout.
*   **Canvas:** HTML5 Canvas for the pixel-art asset editors (rendering the actual NES graphics).
*   **Architecture:** Component-based architecture using native Web Components or simple ES6 modules.

---

*Detailed Language Specification and Roadmap to follow.*

## 4. SwissBASIC Language Specification

SwissBASIC is designed to be familiar to anyone who has used QBASIC or similar dialects, but with first-class citizens for NES hardware concepts (Banks, VBLANK, Interrupts) and inline Assembly.

### 4.1. Core Syntax
*   **Case Sensitivity:** Case-insensitive for keywords, case-sensitive for variables.
*   **Comments:** `REM` or `'` (single quote).
*   **Blocks:** Indentation-agnostic. Uses `BEGIN...END` or specific terminators like `NEXT`, `WEND`, `END IF`.

### 4.2. Data Types
*   `BYTE`: Unsigned 8-bit integer (0-255). The native CPU word size.
*   `WORD`: Unsigned 16-bit integer (0-65535). Used for pointers and memory addresses.
*   `BOOL`: abstraction over a byte (0 = False, >0 = True).
*   **Note:** No native floating point support to maintain performance, though fixed-point libraries can be included.

### 4.3. Key Features
*   **Direct Memory Access:** `PEEK(addr)` and `POKE(addr, val)` are built-in high-performance intrinsics.
*   **Hardware Registers:** Named constants for NES registers (e.g., `PPU_CTRL`, `PPU_MASK`, `JOYPAD1`).
*   **Inline Assembly:**
    ```basic
    ASM
        LDA #$FF
        STA $4011 ; Direct audio write
    END ASM
    ```
*   **Interrupt Handlers:** Special function decorators.
    ```basic
    ON NMI DO VblankRoutine
    ON IRQ DO AudioRoutine
    ```

### 4.4. Example Code
```basic
CONST BG_COLOR = $0F

SUB Main()
    ' Initialize PPU
    POKE(PPU_CTRL, %10000000) ' Enable NMI

    WHILE 1
        WaitFrame()
        ReadController()
        IF Joypad.A THEN
            PlaySound(JUMP_SFX)
        END IF
    WEND
END SUB

INTERRUPT NMI()
    ' DMA Sprite transfer
    CopySprites()
END INTERRUPT
```

## 5. IDE & Asset Editor Specifications

The IDE is a Single Page Application (SPA) composed of several "Workspaces".

### 5.1. The Code Editor
*   **Features:** Syntax highlighting for SwissBASIC and 6502 ASM.
*   **Autocomplete:** Context-aware suggestions for NES registers and user variables.
*   **Memory Map Visualization:** A sidebar showing ROM bank usage and RAM allocation.

### 5.2. The Pattern Editor (Sprite/Tile)
*   **Grid:** 8x8 and 16x16 pixel editing grids.
*   **Tools:** Pencil, Fill Bucket, Shift (Up/Down/Left/Right), Flip (H/V).
*   **Bitplanes:** View separate bitplanes to debug color combining.

### 5.3. The Nametable Editor (Maps)
*   **Canvas:** Large scrolling canvas to paint 8x8 tiles onto a 32x30 (or larger) screen grid.
*   **Metatiles:** Support for defining 16x16 or 32x32 logic blocks composed of smaller tiles.
*   **Collision Layers:** A toggleable overlay to draw collision boxes directly on the map.

### 5.4. The Palette Editor
*   **System Palette:** Visual selector of the 64 hardcoded NES colors.
*   **Sub-palettes:** Interface to assign colors to the 4 background and 4 sprite sub-palettes.
*   **Hex View:** Display hex codes for easy copy-pasting.

### 5.5. Emulator Integration
*   **Core:** A WASM-compiled NES emulator (e.g., a port of a Rust emulator like *Pinky* or custom).
*   **Debug:** Real-time view of CPU registers, Zero Page memory, and VRAM content.


## 6. Implementation Roadmap (30 Phases)

This roadmap assumes an iterative agile approach, verifying functionality at each step.

### Phase 1: Project Initialization
*   Initialize the Rust cargo project structure.
*   Set up the basic HTTP server (Axum/Actix) to serve "Hello World".
*   Configure CI/CD pipelines (linting, basic unit tests).

### Phase 2: Frontend Foundation
*   Create the HTML5 entry point and CSS framework (Vector/SVG focus).
*   Implement a basic "View Router" in vanilla JS to switch between IDE tabs (Code, Graphics, Settings).

### Phase 3: The Lexer (Compiler Part 1)
*   Define the full grammar for SwissBASIC.
*   Implement the Rust Lexer to tokenize the input source code into a stream of tokens.
*   Unit test token generation for all keywords and symbols.

### Phase 4: The AST Definition (Compiler Part 2)
*   Define Rust Enum structures for the Abstract Syntax Tree (AST).
*   Create data structures for Expressions, Statements, Functions, and Loops.

### Phase 5: The Parser (Compiler Part 3)
*   Implement a Recursive Descent Parser in Rust.
*   Connect the Lexer to the Parser to generate ASTs from string inputs.
*   Handle syntax errors with line number reporting.

### Phase 6: Symbol Table & Scope Analysis
*   Implement a Symbol Table to track variable names, types, and memory locations.
*   Implement basic scope checking (global vs local variables).

### Phase 7: Code Generation - Variables
*   Implement the backend to translate variable assignments into 6502 Assembly.
*   Handle 8-bit math (ADD, SUB) logic in assembly generation.

### Phase 8: Code Generation - Control Flow
*   Implement assembly generation for `IF/THEN/ELSE`, `WHILE`, and `FOR` loops.
*   Implement Label generation for jumps and branches.

### Phase 9: Inline Assembler Support
*   Implement the pass-through logic to embed raw ASM blocks directly into the output stream.
*   Validate that ASM syntax is basic-compliant (or pass it raw to the assembler).

### Phase 10: The Assembler Integration
*   Integrate a 6502 Assembler (e.g., *asm6f* crate or custom logic) to turn the generated Assembly into binary `.nes` files.
*   Generate the iNES header.

### Phase 11: End-to-End "Hello World"
*   Create a "Hello World" SwissBASIC program (e.g., changing background color).
*   Verify it compiles to a valid ROM that runs in an external emulator.

### Phase 12: Web Code Editor (Frontend)
*   Implement a text area with line numbers in the web UI.
*   Implement basic syntax highlighting (using regex or a lightweight tokenizer in JS).

### Phase 13: Compiler API
*   Create a POST endpoint `/api/compile` on the Rust server.
*   Connect the Frontend Editor to the Backend Compiler.
*   Return compilation errors or the downloadable `.nes` file.

### Phase 14: Project Management System
*   Implement Backend logic to Create/Save/Load projects (JSON metadata).
*   Implement Frontend UI for a "Project Explorer" sidebar.

### Phase 15: Asset Data Structures
*   Define the JSON schema for storing Sprites (CHR data), Palettes, and Maps in the project file.
*   Implement Rust structs to serialize/deserialize this data.

### Phase 16: Palette Editor UI
*   Build the Frontend interface for selecting NES colors.
*   Implement logic to generate the binary palette data for the compiler.

### Phase 17: CHR (Tile) Editor Core
*   Build the 8x8 pixel grid editor component in JS (Canvas).
*   Implement mouse interaction (draw, erase) and data model updating.

### Phase 18: CHR Editor Tools
*   Add Flood Fill, Shift, and Flip tools to the Tile Editor.
*   Add a "Bank View" to see all 256 tiles in the current set.

### Phase 19: Nametable (Map) Editor Core
*   Build the Map Editor Canvas.
*   Allow placing tiles from the CHR bank onto the Map grid.

### Phase 20: Nametable Editor - Attributes
*   Implement the "Attribute Table" logic (assigning palettes to 16x16 block regions).
*   Visualize palette regions on the map.

### Phase 21: Asset Compilation
*   Update the Compiler to accept asset data (CHR, Palettes, Maps).
*   Generate the assembly code to include these binaries (`.incbin`) and load them into PPU memory during startup.

### Phase 22: Audio Engine - Basics
*   Design a lightweight Sound Engine in 6502 Assembly (to be included in every ROM).
*   Expose SwissBASIC commands to trigger sounds (`PLAY_SFX(id)`).

### Phase 23: Audio Tracker UI
*   Create a basic tracker interface in the Frontend (piano roll or numerical sequencer).
*   Allow defining simple square/triangle wave envelopes.

### Phase 24: Audio Compilation
*   Convert Tracker data into the byte-stream format required by the Audio Engine.
*   Integrate audio data into the ROM build.

### Phase 25: Emulator Integration (WASM)
*   Select an open-source Rust NES emulator.
*   Compile the emulator core to WebAssembly.
*   Embed the WASM emulator in the Frontend "Run" tab.

### Phase 26: Debugging Interface - CPU
*   Create a UI overlay in the Emulator view to show CPU Registers (A, X, Y, PC, SP).
*   Implement a "Step" button to cycle the emulator one instruction at a time.

### Phase 27: Debugging Interface - Memory
*   Create a Hex Editor view of the running emulator's RAM.
*   Allow "Live Peeking" (watching values change).

### Phase 28: Optimization & Polishing - Compiler
*   Implement basic Peephole Optimizations in the compiler (remove redundant loads/saves).
*   Improve error messages with precise line/column pointing.

### Phase 29: Optimization & Polishing - UI
*   Refine the Vector Graphics theme.
*   Ensure the IDE is responsive (works on smaller screens/tablets).
*   Add keyboard shortcuts (Ctrl+S to save, F5 to compile/run).

### Phase 31: Documentation & Examples
*   Write the "SwissBASIC Reference Manual".
*   Create 3-5 example projects (Pong clone, Platformer demo).
*   Include these examples as "New Project" templates.

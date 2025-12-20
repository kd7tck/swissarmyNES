# SwissArmyNES Master Roadmap

**Vision Statement**
SwissArmyNES aims to be the definitive, all-in-one web-based development environment for the Nintendo Entertainment System. It will democratize retro game development by providing a zero-setup, integrated workflow that covers every aspect of the pipeline: from code to assets, audio, debugging, and final ROM generation. The goal is to allow a user to open a browser and have a complete, professional-grade studio at their fingertips.

This document outlines the step-by-step path to achieving that vision, broken down into distinct, testable phases. Before starting a phase check how much of that phase might already be implemented, also when making huge changes to codebase make sure to update other components that might have become broken due to changes.

---
Before starting, Read AGENTS.MD and adhrere to it in a strict manner.

## Phase 1: Foundation & Roadmap (Completed)
**Goal:** Establish the long-term vision and ensure the project structure is ready for expansion.
- **Action Items:**
    - Create this `DESIGN.md` file.
    - Audit current codebase for technical debt (e.g., hardcoded paths, unused modules).
    - Establish `AGENTS.md` to guide AI contributors.
- **Completion Criteria:** `DESIGN.md` exists in root. Codebase passes `cargo clippy` and `cargo fmt`.

## Phase 2: String Data Support (Completed)
**Goal:** Enable basic text handling in SwissBASIC.
- **Action Items:**
    - Add `STRING` data type to AST and Symbol Table.
    - Implement string literal parsing in Lexer/Parser.
    - Update CodeGen to store string data in a dedicated RO-DATA segment.
- **Completion Criteria:** Can declare `DIM name AS STRING = "Hello"` and compile without errors.

## Phase 3: Data Tables (DATA/READ) (Completed)
**Goal:** Allow users to define and access large lookup tables (sine waves, enemy stats) easily.
- **Action Items:**
    - Implement `DATA` statement (stores bytes/words in sequence).
    - Implement `READ` statement (reads from DATA into variables).
    - Implement `RESTORE` statement (resets read pointer).
- **Completion Criteria:** A program can iterate through a `DATA` block and print values to memory.

## Phase 4: Multi-File Projects (Completed)
**Goal:** Support larger codebases by splitting code across multiple files.
- **Action Items:**
    - Implement `INCLUDE "filename.swiss"` directive.
    - Update Parser to recursively load and parse included files.
    - Update Frontend to manage multiple files in the Project Explorer.
- **Completion Criteria:** A `main.swiss` can call a subroutine defined in `lib.swiss`.

## Phase 5: 16-bit Math Expansion (Completed)
**Goal:** Break the 8-bit barrier for general arithmetic.
- **Action Items:**
    - Update CodeGen to support 16-bit addition/subtraction for `WORD` variables.
    - Implement carry-handling logic (ADC/SBC with high bytes).
- **Completion Criteria:** `DIM w AS WORD: LET w = 1000 + 500` results in `1500` at runtime.

## Phase 6: Advanced Math (Mul/Div/Signed) (Completed)
**Goal:** Support complex game logic calculations.
- **Action Items:**
    - Implement 16-bit multiplication and division subroutines (in assembly standard lib).
    - Add `INT` type for signed 8-bit integers.
    - Update expression evaluator to handle signed comparisons.
- **Completion Criteria:** `LET w = 200 * 50` compiles and runs correctly. `-5 < 10` evaluates to true.

## Phase 7: Switch/Case Statement (Completed)
**Goal:** cleaner state machine logic for game loops.
- **Action Items:**
    - Add `SELECT CASE` / `CASE` / `END SELECT` to grammar.
    - CodeGen: Compile to a series of `CMP` / `BEQ` jumps.
- **Completion Criteria:** Replaces nested `IF/ELSEIF` chains in a test program.

## Phase 8: Structs (Records) (Completed)
**Goal:** Group related data (e.g., Player.x, Player.y, Player.hp).
- **Action Items:**
    - Add `TYPE` / `END TYPE` syntax.
    - Implement dot-notation access in Parser (`player.x`).
    - Symbol Table tracks struct layout and offsets.
- **Completion Criteria:** `DIM p AS Player` allocates correct total bytes.

## Phase 9: Enums & Constants (Completed)
**Goal:** Eliminate magic numbers.
- **Action Items:**
    - Implement `ENUM` block syntax.
    - Auto-assign incrementing values to enum members.
- **Completion Criteria:** `CONST STATE_IDLE = 0` can be replaced with `ENUM State { Idle, Run }`.

## Phase 10: Macros (Completed)
**Goal:** Reduce code duplication in SwissBASIC.
- **Action Items:**
    - Implement `DEF MACRO name(args)` syntax.
    - Pre-processor pass to expand macros before parsing.
- **Completion Criteria:** Macro expansion works for simple code blocks.

---

## Phase 11: Standard Library - Controller (Completed)
**Goal:** Simplify input handling.
- **Action Items:**
    - Create built-in `Controller` module.
    - Methods: `Read()`, `IsPressed(btn)`, `IsHeld(btn)`, `IsReleased(btn)`.
- **Completion Criteria:** User code: `IF Controller.IsPressed(Start) THEN ...`.

## Phase 12: Standard Library - Text Engine (Completed)
**Goal:** Easy string rendering to screen.
- **Action Items:**
    - `Print(x, y, string)` routine.
    - Support for custom font mappings (ASCII to Tile Index).
- **Completion Criteria:** "Hello World" printed to nametable at (10, 10).

## Phase 13: Metasprite System (Completed)
**Goal:** Handle sprites larger than 8x8.
- **Action Items:**
    - Define `Metasprite` data structure (list of relative x, y, tile, attr).
    - `DrawMetasprite(x, y, id)` routine in Assembly.
    - OAM cycling/flickering logic to handle > 8 sprites per line.
- **Completion Criteria:** Rendering a 16x32 character composed of 8 sprites.

## Phase 14: Animation Engine (Completed)
**Goal:** Animate Metasprites easily.
- **Action Items:**
    - Define `Animation` structure (frames, duration, loop).
    - State machine to update current frame based on timer.
- **Completion Criteria:** Character walks (cycles 3 frames) when moving.

## Phase 15: Object Pooling
**Goal:** Manage game entities dynamically without dynamic memory allocation.
- **Action Items:**
    - Implement a static `Entity` pool array.
    - Methods: `Spawn()`, `Despawn()`.
- **Completion Criteria:** Can spawn 10 enemies, kill one, and spawn a new one in its slot.

## Phase 16: Collision - AABB (Completed)
**Goal:** Basic box collision.
- **Action Items:**
    - `CheckCollision(obj1, obj2)` function.
    - Optimized 8-bit or 16-bit comparison logic in ASM.
- **Completion Criteria:** Returns true when two defined rectangles overlap.

## Phase 17: Collision - Point & Tile (Completed)
**Goal:** Wall/Floor collisions.
- **Action Items:**
    - `GetTileAt(pixel_x, pixel_y)` function.
    - Map pixel coordinates to Nametable indices.
- **Completion Criteria:** Player stops when walking into a "solid" tile ID.

## Phase 18: Scrolling - Horizontal (In Progress)
**Goal:** Enable side-scrollers.
- **Action Items:**
    - Implement camera variable logic.
    - "Seam" update logic (writing new columns to Nametable as camera moves).
    - Attribute table handling for scrolling (tricky 2-bit logic).
- **Completion Criteria:** Map scrolls infinitely to the right without artifacts.

## Phase 19: Scrolling - Vertical
**Goal:** Enable vertical shooters/platformers.
- **Action Items:**
    - Vertical camera logic.
    - Y-scroll register split handling (if status bar exists).
- **Completion Criteria:** Map scrolls up/down correctly.

## Phase 20: Random Number Generator
**Goal:** Procedural generation and unpredictable gameplay.
- **Action Items:**
    - Implement a fast 8-bit or 16-bit Linear Congruential Generator (LCG) or LFSR.
    - Seed initialization function.
- **Completion Criteria:** `RND(100)` returns varied numbers 0-99.

---

## Phase 21: Audio - DPCM Support
**Goal:** Play sampled sounds (drums, voice).
- **Action Items:**
    - Audio Compiler: Support DMC channel data.
    - Add sample import (WAV -> 1-bit Delta) tool.
- **Completion Criteria:** A voice sample plays on button press.

## Phase 22: Audio - SFX Priority
**Goal:** Prevent sound effects from cutting off music awkwardly.
- **Action Items:**
    - Implement priority flag for SFX tracks.
    - Sound engine only interrupts music channel if SFX priority > current note priority.
- **Completion Criteria:** Shooting sound plays over music, but low-priority ambient noise doesn't kill melody.

## Phase 23: Audio - Envelope Editor UI
**Goal:** Visual sound design.
- **Action Items:**
    - Create a graph editor for Volume and Pitch envelopes.
    - Export envelope data to Audio Compiler format.
- **Completion Criteria:** User draws a fade-out curve, engine reproduces it.

## Phase 24: Audio - Arpeggios
**Goal:** Chiptune chords.
- **Action Items:**
    - Add "Arp" macro support to Tracker.
    - Engine cycles pitch offsets every frame (0, 4, 7 semitones).
- **Completion Criteria:** A single channel produces a major chord sound.

## Phase 25: Audio - Import
**Goal:** Migrate from other tools.
- **Action Items:**
    - Importer for FamiTracker text export or generic MIDI.
    - Map MIDI channels to NES Pulse/Tri.
- **Completion Criteria:** Drag-and-drop a `.txt` file to populate the Tracker.

---

## Phase 26: Visual - CHR Import
**Goal:** Use external art tools.
- **Action Items:**
    - JS-based PNG parser.
    - Color quantization (nearest neighbor to NES palette).
    - Slice 128x128 image into 8x8 tiles.
- **Completion Criteria:** Dragging a PNG onto CHR Editor populates the bank.

## Phase 27: Visual - Metatile Editor
**Goal:** Build maps faster.
- **Action Items:**
    - Create UI for defining 16x16 or 32x32 blocks (composed of 4 or 16 tiles).
    - Assign palette attributes to the metatile.
- **Completion Criteria:** User paints with 16x16 blocks instead of single tiles.

## Phase 28: Visual - Screen Editor
**Goal:** Organize game world.
- **Action Items:**
    - Define "Screen" as a collection of Metatiles (e.g., 16x15 blocks).
    - Store screens as compressed data arrays.
- **Completion Criteria:** Can edit and save 10 distinct screens.

## Phase 29: Visual - World Editor
**Goal:** Stitch screens together.
- **Action Items:**
    - 2D Grid view of Screens.
    - Define adjacency (which screen is Right/Up/Down/Left).
- **Completion Criteria:** Visual map of the entire game world.

## Phase 30: Visual - Sprite Editor
**Goal:** Dedicated character design tool.
- **Action Items:**
    - Canvas allowing free placement of 8x8 sprite tiles.
    - Timeline UI for animation frames.
    - Export to `Metasprite` data format (Phase 13).
- **Completion Criteria:** Create a "Walking Link" animation in UI and use it in code.

---

## Phase 31: Emulator - WASM Integration
**Goal:** Run the game without leaving the browser.
- **Action Items:**
    - Select a Rust NES emulator crate (e.g., `pinky`, `nes-rs`) or write a minimal one.
    - Compile emulator to WASM.
    - Integrate WASM module into `editor.js`.
- **Completion Criteria:** Clicking "Run" opens a canvas overlay playing the game.

## Phase 32: Emulator - UI Wrapper
**Goal:** User controls for the emulator.
- **Action Items:**
    - Play/Pause/Reset buttons.
    - 1x/2x/Full Screen scaling.
    - Volume control.
- **Completion Criteria:** UI controls functional.

## Phase 33: Emulator - Input
**Goal:** Play the game.
- **Action Items:**
    - Map Browser `keydown`/`keyup` to NES Controller byte.
    - Gamepad API integration for USB controllers.
- **Completion Criteria:** Xbox controller moves the character in the browser.

## Phase 34: Debugging - Protocol
**Goal:** Communication between Editor and running Emulator.
- **Action Items:**
    - Define shared memory or message passing interface (WASM <-> JS).
    - Expose CPU RAM and Registers to JS.
- **Completion Criteria:** JS can read PC (Program Counter) from running WASM.

## Phase 35: Debugging - Source Maps
**Goal:** Relate high-level code to Assembly.
- **Action Items:**
    - Compiler emits a `sourcemap.json` (Line Number -> ROM Address range).
    - ASM emits Address -> Source Line.
- **Completion Criteria:** Hovering a line in Editor shows the corresponding Memory Address.

## Phase 36: Debugging - Breakpoints
**Goal:** Stop and inspect.
- **Action Items:**
    - Click gutter in Editor to set breakpoint (Line #).
    - Emulator checks PC against breakpoint list every instruction.
    - Pause execution when hit.
- **Completion Criteria:** Game freezes when execution hits line 10.

## Phase 37: Debugging - Memory Viewer
**Goal:** Inspect state.
- **Action Items:**
    - Hex Editor UI component (virtualized list for performance).
    - Live updates (poll every frame or on pause).
- **Completion Criteria:** Can see variable values change in RAM at `$0300`.

## Phase 38: Debugging - PPU Viewer
**Goal:** Graphics debugging.
- **Action Items:**
    - View Pattern Tables (CHR) as interpreted by PPU.
    - View Nametables (scrolled view).
    - View OAM (Sprite list).
- **Completion Criteria:** Can see off-screen enemies in the visualizer.

---

## Phase 39: Mappers - MMC1
**Goal:** Larger games (up to 256KB).
- **Action Items:**
    - Assembler/Linker support for bank switching.
    - Compiler directives for `BANK 0`, `BANK 1`.
    - Mapper #1 initialization code.
- **Completion Criteria:** A 128KB ROM runs correctly.

## Phase 40: Mappers - MMC3
**Goal:** Advanced raster effects and huge games.
- **Action Items:**
    - Mapper #4 support.
    - IRQ handler abstraction (`ON SCANLINE 100 DO ...`).
- **Completion Criteria:** Split-screen scrolling effect works.

## Phase 41: Optimization - Peephole
**Goal:** Faster/Smaller code.
- **Action Items:**
    - Analyze generated ASM for redundant patterns (e.g., `LDA #0; STA $Var; LDA #0`).
    - Remove redundant loads/stores.
- **Completion Criteria:** Benchmark program runs 10% faster.

## Phase 42: Optimization - Liveness Analysis
**Goal:** Reduce RAM usage.
- **Action Items:**
    - Analyze variable life-spans.
    - Re-use memory addresses for variables that don't overlap in scope.
- **Completion Criteria:** Large program fits in 2KB RAM where it previously failed.

## Phase 43: Interactive Tutorials
**Goal:** Onboarding new users.
- **Action Items:**
    - Overlay system highlighting UI elements.
    - Step-by-step "Make your first game" guide inside the IDE.
- **Completion Criteria:** A user can follow prompt bubbles to compile "Hello World".

## Phase 44: Asset Library
**Goal:** Reusability.
- **Action Items:**
    - "My Assets" panel.
    - Drag and drop assets between projects.
    - Standard library of common assets (fonts, basic sprites).
- **Completion Criteria:** Import a font from the global library.

## Phase 45: Desktop Build
**Goal:** Offline development.
- **Action Items:**
    - Wrap the application in Tauri (Rust-based Electron alternative).
    - Local file system access (bypass browser sandbox).
- **Completion Criteria:** Standalone `.exe` / `.app` runs without internet.

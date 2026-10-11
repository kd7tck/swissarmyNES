# SwissArmyNES Master Roadmap

Build an integrated NES platform for game design, code, art/worlds, game flow, testing, music and SFX. Keep Rust/Axum, vanilla browser code and the shared production core. Inspect existing behavior before implementing a phase.

**October 11 checkpoint:** Studio scenes, entities, variables/events and controller scenarios are implemented. Local 350-test verification and browser evidence are in [resume state](docs/IMPLEMENTATION_RESUME.md). Next work follows the [Studio roadmap](docs/GAME_STUDIO_ROADMAP.md); original 1980s ROM recovery is a later Phase 48 goal.

Phase numbering and original completion criteria are retained below. Historical Completed labels are not release signoffs: use the [acceptance worksheet](docs/PHASE_40_ACCEPTANCE_CHECKLIST.md). The [detailed handoff](docs/DEVELOPER_HANDOFF_THROUGH_PHASE_40.md) retains step-by-step instructions; the [baseline audit](docs/BASELINE_AUDIT.md) records historical failures. [Runtime contracts](docs/RUNTIME_CONTRACTS.md) describe current implementation, including production compilation, bank/RAM limits, source mapping, interrupts and cartridge behavior. Read [AGENTS](AGENTS.md) before development.

## Phase 1: Foundation & Roadmap (Completed)

**Goal:** Establish the long-term vision and ensure the project structure is ready for expansion.

**Work:** Create this `DESIGN.md` file. Audit current codebase for technical debt (e.g., hardcoded paths, unused modules). Establish `AGENTS.md` to guide AI contributors.

**Gate:** `DESIGN.md` exists in root. Codebase passes `cargo clippy` and `cargo fmt`.

## Phase 2: String Data Support (Completed)

**Goal:** Enable basic text handling in SwissBASIC.

**Work:** Add `STRING` data type to AST and Symbol Table. Implement string literal parsing in Lexer/Parser. Update CodeGen to store string data in a dedicated RO-DATA segment.

**Gate:** Can declare `DIM name AS STRING = "Hello"` and compile without errors.

## Phase 3: Data Tables (DATA/READ) (Completed)

**Goal:** Allow users to define and access large lookup tables (sine waves, enemy stats) easily.

**Work:** Implement `DATA` statement (stores bytes/words in sequence). Implement `READ` statement (reads from DATA into variables). Implement `RESTORE` statement (resets read pointer).

**Gate:** A program can iterate through a `DATA` block and print values to memory.

## Phase 4: Multi-File Projects (Completed)

**Goal:** Support larger codebases by splitting code across multiple files.

**Work:** Implement `INCLUDE "filename.swiss"` directive. Update Parser to recursively load and parse included files. Update Frontend to manage multiple files in the Project Explorer.

**Gate:** A `main.swiss` can call a subroutine defined in `lib.swiss`.

## Phase 5: 16-bit Math Expansion (Completed)

**Goal:** Break the 8-bit barrier for general arithmetic.

**Work:** Update CodeGen to support 16-bit addition/subtraction for `WORD` variables. Implement carry-handling logic (ADC/SBC with high bytes).

**Gate:** `DIM w AS WORD: LET w = 1000 + 500` results in `1500` at runtime.

## Phase 6: Advanced Math (Mul/Div/Signed) (Completed)

**Goal:** Support complex game logic calculations.

**Work:** Implement 16-bit multiplication and division subroutines (in assembly standard lib). Add `INT` type for signed 8-bit integers. Update expression evaluator to handle signed comparisons.

**Gate:** `LET w = 200 * 50` compiles and runs correctly. `-5 < 10` evaluates to true.

## Phase 7: Switch/Case Statement (Completed)

**Goal:** cleaner state machine logic for game loops.

**Work:** Add `SELECT CASE` / `CASE` / `END SELECT` to grammar. CodeGen: Compile to a series of `CMP` / `BEQ` jumps.

**Gate:** Replaces nested `IF/ELSE` chains in a test program.

## Phase 8: Structs (Records) (Completed)

**Goal:** Group related data (e.g., Player.x, Player.y, Player.hp).

**Work:** Add `TYPE` / `END TYPE` syntax. Implement dot-notation access in Parser (`player.x`). Symbol Table tracks struct layout and offsets.

**Gate:** `DIM p AS Player` allocates correct total bytes.

## Phase 9: Enums & Constants (Completed)

**Goal:** Eliminate magic numbers.

**Work:** Implement `ENUM` block syntax. Auto-assign incrementing values to enum members.

**Gate:** `CONST STATE_IDLE = 0` can be replaced with `ENUM State { Idle, Run }`.

## Phase 10: Macros (Completed)

**Goal:** Reduce code duplication in SwissBASIC.

**Work:** Implement `DEF MACRO name(args)` syntax. Pre-processor pass to expand macros before parsing.

**Gate:** Macro expansion works for simple code blocks.

## Phase 11: Standard Library - Controller (Completed)

**Goal:** Simplify input handling.

**Work:** Create built-in `Controller` module. Methods: `Read()`, `IsPressed(btn)`, `IsHeld(btn)`, `IsReleased(btn)`.

**Gate:** User code: `IF Controller.IsPressed(Start) THEN ...`.

## Phase 12: Standard Library - Text Engine (Completed)

**Goal:** Easy string rendering to screen.

**Work:** `Print(x, y, string)` routine. Support for custom font mappings (ASCII to Tile Index).

**Gate:** "Hello World" printed to nametable at (10, 10).

## Phase 13: Metasprite System (Completed)

**Goal:** Handle sprites larger than 8x8.

**Work:** Define `Metasprite` data structure (list of relative x, y, tile, attr). `DrawMetasprite(x, y, id)` routine in Assembly. OAM cycling/flickering logic to handle > 8 sprites per line.

**Gate:** Rendering a 16x32 character composed of 8 sprites.

## Phase 14: Animation Engine (Completed)

**Goal:** Animate Metasprites easily.

**Work:** Define `Animation` structure (frames, duration, loop). State machine to update current frame based on timer.

**Gate:** Character walks (cycles 3 frames) when moving.

## Phase 15: Object Pooling (Completed)

**Goal:** Manage game entities dynamically without dynamic memory allocation.

**Work:** Implement a static `Entity` pool array. Methods: `Spawn()`, `Despawn()`.

**Gate:** Can spawn 10 enemies, kill one, and spawn a new one in its slot.

## Phase 16: Collision - AABB (Completed)

**Goal:** Basic box collision.

**Work:** `CheckCollision(obj1, obj2)` function. Optimized 8-bit or 16-bit comparison logic in ASM.

**Gate:** Returns true when two defined rectangles overlap.

## Phase 17: Collision - Point & Tile (Completed)

**Goal:** Wall/Floor collisions.

**Work:** `GetTileAt(pixel_x, pixel_y)` function. Map pixel coordinates to Nametable indices.

**Gate:** Player stops when walking into a "solid" tile ID.

## Phase 18: Scrolling - Horizontal (Completed)

**Goal:** Enable side-scrollers.

**Work:** Implement camera variable logic (`Scroll` module). "Seam" update logic (via `Scroll.LoadColumn` VBlank buffer). Attribute table handling (via `PPU` control).

**Gate:** Map scrolls infinitely to the right without artifacts.

## Phase 19: Scrolling - Vertical (Completed)

**Goal:** Enable vertical shooters/platformers.

**Work:** Vertical camera logic. Y-scroll register split handling (if status bar exists).

**Gate:** Map scrolls up/down correctly.

## Phase 20: Random Number Generator (Completed)

**Goal:** Procedural generation and unpredictable gameplay.

**Work:** Implement a fast 8-bit or 16-bit Linear Congruential Generator (LCG) or LFSR. Seed initialization function.

**Gate:** `RND(100)` returns varied numbers 0-99.

## Phase 21: Audio - DPCM Support (Completed)

**Goal:** Play sampled sounds (drums, voice).

**Work:** Audio Compiler: Support DMC channel data. Add sample import (WAV -> 1-bit Delta) tool.

**Gate:** A voice sample plays on button press.

## Phase 22: Audio - SFX Priority (Completed)

**Goal:** Prevent sound effects from cutting off music awkwardly.

**Work:** Implement priority flag for SFX tracks. Sound engine only interrupts music channel if SFX priority > current note priority.

**Gate:** Shooting sound plays over music, but low-priority ambient noise doesn't kill melody.

## Phase 23: Audio - Envelope Editor UI (Completed)

**Goal:** Visual sound design.

**Work:** Create a graph editor for Volume and Pitch envelopes. Export envelope data to Audio Compiler format.

**Gate:** User draws a fade-out curve, engine reproduces it.

## Phase 24: Audio - Arpeggios (Completed)

**Goal:** Chiptune chords.

**Work:** Add "Arp" macro support to Tracker. Engine cycles pitch offsets every frame (0, 4, 7 semitones).

**Gate:** A single channel produces a major chord sound.

## Phase 25a: Audio - SFX Engine Core (Completed)

**Goal:** Define the binary format for Sound Effects and implement the runtime playback logic (Assembler/ASM).

**Work:** Define `SoundEffect` struct in Rust (Channel, Priority, Speed, Sequences for Vol/Pitch/Duty). Implement `SFX_Play` routine in ASM that supports playing these "macro-based" effects. Ensure SFX engine handles priority correctly.

**Gate:** Manually defined SFX data can be compiled and played via `PLAY_SFX` in a test ROM.

## Phase 25b: Audio - SFX Editor UI Foundation (Completed)

**Goal:** Create the dedicated workspace for Sound Effect creation, separate from the Music Tracker, rename Audio tab to music to make sure there is no future confusion.

**Work:** Add "SFX" tab to the Audio Editor. Implement list management (Create, Rename, Delete SFX). Basic properties panel (Channel selection, Loop settings).

**Gate:** User can create a named SFX entry and save the project.

## Phase 25c: Audio - Visual Envelope Designers (Completed)

**Goal:** Intuitive visual editing of sound parameters over time.

**Work:** Implement "Draw" canvas for Volume envelopes. Implement "Draw" canvas for Pitch sweeps and Duty cycles. Implement "Arpeggio" macro editor (specific sequence of note offsets).

**Gate:** User can draw a volume fade-out curve and a pitch slide visually.

## Phase 25d: Audio - Import/Export & Full Integration (Completed)

**Goal:** Asset sharing and final polish.

**Work:** Implement JSON Import/Export for individual SFX patches. Implement Drag-and-Drop support to load SFX files. Hook up the Compile pipeline to generate the binary data from the UI.

**Gate:** Drag-and-drop a `.sfx.json` file to populate the GUI, compile, and hear the sound in the game.

## Phase 26: Visual - CHR Import (Completed)

**Goal:** Use external JS art tools.

**Work:** JS-based PNG parser. Color quantization (nearest neighbor to NES palette). Slice 128x128 image into 8x8 tiles.

**Gate:** Dragging a PNG onto CHR Editor populates the bank.

## Phase 27: Visual - Metatile Editor (Completed)

**Goal:** Build maps faster.

**Work:** Create UI for defining 16x16 or 32x32 blocks (composed of 4 or 16 tiles). Assign palette attributes to the metatile.

**Gate:** User paints with 16x16 blocks instead of single tiles.

## Phase 28: Visual - Metatile Integration (Completed)

**Goal:** Organize game world. Screen editing will be built into the map editor directly. Allowing a person to edit individial tiles or collection of tiles called metatiles.

**Work:** Integrate metatiles into MAP UI system more fully.

**Gate:** Can edit and work with metatiles in other systems that also use tiles.

## Phase 29: Visual - World Editor for MAPS (Completed)

**Goal:** Stitch Maps together. each grid point in world editor is a map from the map editor.

**Work:** 2D Grid view of tiles or metatiles in vast grid map able to be stored in ROM. Define adjacency (which MAP is Right/Up/Down/Left).

**Gate:** Visual map of the entire game world. and ability to import maps and use maps inside of world editor.

## Phase 30: Visual - Sprite Editor (Completed)

**Goal:** Dedicated character design tool.

**Work:** Canvas allowing free placement of 8x8 sprite tiles or metatiles. Timeline UI for animation frames. Export to `Metasprite` data format (Phase 13).

**Gate:** Create a "Walking Link" animation in UI and use it in code.

## Phase 31: Emulator - WASM Integration

**Goal:** Run the game without leaving the browser.

**Work:** Build the shared patched TetaNES production core for WASM; retain one native/browser implementation. Compile emulator to WASM. Integrate WASM module into `editor.js`. Share first-run initialization, validate constructor/memory exports together, and discard stale Run requests after edits, project changes or Close. Revalidate unversioned browser assets after updates, including conditional cached responses.

**Gate:** Clicking "Run" opens a canvas overlay playing the game.

## Phase 32: Emulator - UI Wrapper

**Goal:** User controls for the emulator.

**Work:** Play/Pause/Reset buttons. 1x/2x/Full Screen scaling. Volume control. Full scaling reserves space for wrapped toolbar controls so Pause, Reset and Close stay accessible.

**Gate:** UI controls functional.

## Phase 33: Emulator - Input

**Goal:** Play the game.

**Work:** Map Browser `keydown`/`keyup` to NES Controller byte. Gamepad API integration for USB controllers.

**Gate:** Xbox controller moves the character in the browser.

## Phase 34: Debugging - Protocol

**Goal:** Communication between Editor and running Emulator.

**Work:** Define shared memory or message passing interface (WASM <-> JS). Expose CPU RAM and Registers to JS.

**Gate:** JS can read PC (Program Counter) from running WASM.

## Phase 35: Debugging - Source Maps

**Goal:** Relate high-level code to Assembly.

**Work:** Compiler emits a `sourcemap.json` (Line Number -> ROM Address range). ASM emits Address -> Source Line.

**Gate:** Hovering a line in Editor shows the corresponding Memory Address.

## Phase 36: Debugging - Breakpoints (Completed)

**Goal:** Stop and inspect.

**Work:** Click gutter in Editor to set breakpoint (Line #). Emulator checks PC against breakpoint list every instruction. Pause execution when hit.

**Gate:** Game freezes when execution hits line 10.

## Phase 37: Debugging - Memory Viewer (Completed)

**Goal:** Inspect state.

**Work:** Hex Editor UI component (virtualized list for performance). Live updates (poll every frame or on pause).

**Gate:** Can see variable values change in RAM at `$0300`.

## Phase 38: Debugging - PPU Viewer (Completed)

**Goal:** Graphics debugging.

**Work:** View Pattern Tables (CHR) as interpreted by PPU. View Nametables (scrolled view). View OAM (Sprite list).

**Gate:** Can see off-screen enemies in the visualizer.

## Phase 39: Repository Ingestion & Bus/Cartridge Architecture

**Goal:** Establish the foundation for the NES emulator architecture by parsing headers and routing the 16-bit CPU Memory Bus.

**Work:** Parse iNES (.nes) and NES 2.0 header structures (PRG-ROM/RAM sizes, CHR-ROM/RAM sizes, mirroring flags, mapper ID). Define the unified 16-bit CPU Memory Bus routing: 2 KB internal RAM with mirrors ($0000–$1FFF), PPU registers mirrored every 8 bytes ($2000–$3FFF), APU/IO registers ($4000–$4017), and cartridge memory space ($4020–$FFFF). Connect named mapper read/write/IRQ operations to the actual production CPU/PPU bus; validate routing instead of introducing a mock-only abstraction.

**Gate:** ROM headers are parsed and a skeleton Memory Bus is functional.

## Phase 40: Ricoh 2A03 (6502) CPU Core Hardening

**Goal:** Harden and verify the shared Ricoh 2A03 CPU implementation.

**Work:** Implement all 56 official 6502 instructions across all 12 addressing modes. Model hardware interrupts (RESET vector at $FFFC, NMI at $FFFA, IRQ/BRK at $FFFE). Support illegal/unofficial opcodes required by standard test suites (e.g., LAX, SAX, DCP, ISB). Validate using Kevtris’s `nestest.nes` in headless execution mode by piping instruction traces against the canonical reference log (golden log comparing PC, registers A, X, Y, P, SP, and cycle counters).

**Gate:** CPU passes `nestest.nes` validation logs flawlessly.

## Phase 41: Ricoh 2C02 PPU (Picture Processing Unit)

**Goal:** Build an accurate NES picture processing unit for rendering graphics.

**Work:** Implement the internal register state machine: PPUCTRL ($2000), PPUMASK ($2001), PPUSTATUS ($2002), OAMADDR/OAMDATA ($2003/$2004), PPUSCROLL ($2005), PPUADDR ($2006), and PPUDATA ($2007). Model Loopy’s internal VRAM address registers (v, t, x, w) for accurate scrolling and fine-X shifts. Frame timing state machine: 262 scanlines, 341 cycles per scanline (pre-render, visible scanlines 0–239, post-render line 240, VBlank lines 241–260). Sprite evaluation pipeline: primary OAM (64 sprites, 256 bytes), secondary OAM (8 sprites per scanline limit), 8×8 and 8×16 sprite rendering, and Sprite 0 Hit detection. Generate VBlank NMI interrupt to signal the CPU at cycle 1 of scanline 241.

**Gate:** PPU can correctly render a basic static screen and background graphics from standard ROMs.

## Phase 42: Core Memory Mappers

**Goal:** Support various cartridge hardware memory mappers.

**Work:** Mapper 0 (NROM): Zero bank switching; supports classics like Donkey Kong, Super Mario Bros., and Pac-Man. Mapper 1 (MMC1): 5-bit serial write shift register protocol, dynamic PRG/CHR banking, and configurable mirroring (The Legend of Zelda, Metroid, Mega Man 2). Mapper 2 (UxROM): Switchable PRG bank with fixed upper bank (Castlevania, Contra). Mapper 4 (MMC3): Scanline-based IRQ counter for split-screen status bars and mid-frame bank switches (Super Mario Bros. 3, Mega Man 3, Kirby’s Adventure).

**Gate:** Emulator successfully boots and runs games requiring NROM, MMC1, UxROM, and MMC3 logic.

## Phase 43: Ricoh 2A03 APU (Audio Processing Unit)

**Goal:** Add sound processing capabilities to the emulator.

**Work:** Synthesize five channels: Pulse 1 and 2 (with envelope and frequency sweep units), Triangle (with linear counter), Noise (15-bit/9-bit LFSR pseudo-random generator), and DMC (Delta Modulation Channel with direct DMA sample memory reads). Implement the Frame Counter sequencer (4-step 60 Hz and 5-step 48 Hz modes). Stream synchronized browser audio using the existing Web Audio path; any future native host output needs its own architecture and underrun tests.

**Gate:** Basic sound generation for Pulse, Triangle, and Noise channels works correctly.

## Phase 44: The "Swiss Army" Debugging Suite & Frontend

**Goal:** Provide powerful real-time tools for retro developers in the frontend.

**Work:** Extend the existing vanilla-browser inspection overlay; preserve production-core ownership. Pattern Table Viewer: Live palette swapping over 16×16 tile sheets ($0000–$1FFF). Nametable Visualizer: Real-time 4-quadrant scrolling display with viewport bounding box overlay. OAM Sprite Viewer: Inspect coordinates, pattern addresses, and priority flags. CPU Disassembler & Memory Hex Editor: Set execution breakpoints, step single instructions, and patch RAM on the fly. State serialization via serde for instant save states and rewind buffers.

**Gate:** Debugging overlays function in real-time alongside emulator execution.

## Phase 45: Interactive Tutorials

**Goal:** Onboarding new users.

**Work:** Overlay system highlighting UI elements. Step-by-step "Make your first game" guide inside the IDE.

**Gate:** A user can follow prompt bubbles to compile "Hello World".

## Phase 46: Asset Library

**Goal:** Reusability.

**Work:** "My Assets" panel. Drag and drop assets between projects. Standard library of common assets (fonts, basic sprites).

**Gate:** Import a font from the global library.

## Phase 47: Desktop Build

**Goal:** Offline development.

**Work:** Wrap the application in Tauri (Rust-based Electron alternative). Local file system access (bypass browser sandbox).

**Gate:** Standalone `.exe` / `.app` runs without internet.

## Phase 48: ROM Import and Editable Project Recovery (Planned)

**Goal:** Import original 1980s NES/Famicom cartridge game ROMs into complete editable projects, rebuild them and verify broad emulator compatibility.

**Work:** Add Import NES ROM with validated cartridge metadata, bank-aware code/data recovery and a versioned recovered project format. Preserve the original engine, mapper and bank layout; provide an exact reconstruction build mode alongside authored SwissBASIC projects. Recover graphics, maps, animation, game systems, music and SFX into Studio editors through verified profiles and annotations, retaining uncertain regions as editable source with provenance. Prove unchanged source-only rebuilds byte-for-byte, then verify meaningful code/asset edits and gameplay/save behavior on independent emulators. Report recovery coverage and tested emulator/hardware support explicitly; original source identity and universal emulator compatibility are not guaranteed.

**Gate:** Representative original 1980s games across supported cartridge families import, reopen, edit, preview, rebuild and export successfully; untouched rebuilds match their input and edited examples pass the documented emulator matrix. A ROM dump alone is insufficient.

[Detailed recovery plan](docs/ROM_IMPORT_ROADMAP.md). Preserve original mapper/engine, uncertain regions and independent-emulator evidence; no original-source or universal-emulator guarantee.

## Current implementation evidence

Native and generated WASM match all 8,991 canonical CPU records; supplemental CPU/interrupt/DMA tests and atomic banked NMI/IRQ bindings are documented in [runtime contracts](docs/RUNTIME_CONTRACTS.md) and [emulator ownership](docs/EMULATOR_ARCHITECTURE.md). Current idle NMI measurement is 1,136 cycles including DMA; active PPU work/handler timings and limitations are recorded there. Phase 39/40 and complete release acceptance remain subject to the worksheet, physical devices and current host-matrix evidence.

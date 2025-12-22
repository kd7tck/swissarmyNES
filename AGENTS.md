# AI Agent Guidelines

This document serves as the primary instruction manual for AI agents working on the SwissArmyNES codebase. Strictly adhere to these guidelines to ensure consistency, quality, and stability.

## Core Directives

1.  **Strict Design Adherence**: The `DESIGN.md` file is the master roadmap. Do not deviate from the current phase or the specified architecture without explicit user approval.
2.  **Cross-Platform Compatibility**: This is a Rust application. It must compile and run on Windows, macOS, and Linux without modification. Avoid OS-specific commands or paths.
3.  **Documentation**:
    - Update `README.md`, `DESIGN.md`, and this `AGENTS.md` file whenever major changes occur.
    - Do not remove existing comments unless they are factually incorrect or the code they describe is deleted.
    - Add clear, concise comments to all new code, explaining the "why" more than the "what".
4.  **Testing**:
    - "Test Constantly" is the mantra. Run tests after every significant code change.
    - You are permitted to modify existing test files (`tests/`) if your changes legitimately break old tests (e.g., changing an API).
    - Create new tests for every new feature or bug fix.
5.  **Code Quality**:
    - **Formatting**: All code must pass `cargo fmt`.
    - **Linting**: All code must pass `cargo clippy -- -D warnings`. Address all warnings immediately.
    - **Dead Code**: While `dead_code` warnings might be suppressed in CI, strive to minimize unused code.

## Project Structure

-   `src/lib.rs`: The library entry point. Contains the compiler core and server logic modules.
-   `src/main.rs`: The binary entry point. Sets up the Axum server and handles command-line args.
-   `src/compiler/`: All compiler logic (Lexer, Parser, AST, Symbol Table, Codegen, Audio).
-   `src/server/`: Backend logic (API endpoints, project file management).
-   `static/`: The frontend (HTML, CSS, Vanilla JS).
-   `projects/`: Local storage for user projects (git-ignored, but structure matters).
-   `tests/`: Integration tests.

## Development Workflow

1.   **Read Brain section in AGENTS.md**: Pick back up where the last AI agent left off.
2.  **Plan**: Read the requirements, verify the state of the code, and formulate a plan using `set_plan`.
3.  **Edit**: Make changes.
4.  **Verify**:
    - Run `cargo check` to catch compilation errors early.
    - Run `cargo fmt` to fix style.
    - Run `cargo clippy` to ensure quality.
    - Run `cargo test` to verify functionality.
5.  **Refine**: If tests fail, diagnose and fix. Do not guess. Use `read_file` to see the actual code.
6.  **Store current state in Brain section of AGENTS.md**: Help the next AI agent out by telling them what has been done and what should be done next, allong with pitfalls to look out for.
7.  **Submit**: Only when all checks pass.

## Technical specifics

-   **Assembler**: The project uses `rs6502` for assembly generation. Be aware of its limitations (e.g., no `.BYTE` directive support, use `db` equivalent or injection).
-   **Frontend**: Plain HTML/JS/CSS. No build step (Webpack/Vite) is currently used. Keep it simple.
-   **Memory Management**: The NES has 2KB of RAM. The compiler must manage this strictly (`$0000-$07FF`).

## Brain
Phase 31, 32, and 33 are complete.

### Phase 31: Emulator - WASM Integration (Completed)
- **Implemented**: `swiss-emulator` crate in `emulator/` directory using `tetanes-core`.
- **Compiling**: `emulator` compiles to `wasm32-unknown-unknown` and exposes `Emulator` class via `wasm-bindgen`.
- **Frontend**:
    - `editor.js` handles loading the WASM module dynamically.
    - `editor.js` creates a Canvas overlay when "Run (Emulator)" is clicked.
    - `app.js` listens for `request-compile-and-run` event, compiles source via API, and passes ROM bytes to `editor.js`.
- **Key Features**:
    - **WASM Module**: Exposes `load_rom`, `step`, `get_pixels`, `set_button`, `reset`, `set_sample_rate`, `get_audio_samples`.
    - **Input**: Maps Z/X (A/B), Shift (Select), Enter (Start), Arrows (D-Pad).

### Phase 32: Emulator - UI Wrapper (Completed)
- **Implemented**: Emulator Overlay in `editor.js`.
- **Features**:
    - **Controls**: Play/Pause button (toggles loop), Reset button.
    - **Scaling**: 1x, 2x, Fullscreen buttons.
    - **Volume**: Slider controlling Web Audio GainNode.
    - **Integration**: `app.js` now handles the "Run" button by compiling the project (without downloading) and dispatching `emulator-load-rom`.

### Phase 33: Emulator - Input (Completed)
- **Implemented**: Gamepad support in `static/js/editor.js`.
- **Features**:
    - **Gamepad Polling**: Checks `navigator.getGamepads()` every frame in `emulatorLoop`.
    - **Mapping**: Xbox A -> NES A, Xbox B/X -> NES B, Back -> Select, Start -> Start, D-Pad/Axes -> D-Pad.
    - **Input Mixing**: Keyboard and Gamepad inputs are merged (Logical OR), allowing simultaneous use.
    - **State Management**: Updates are only sent to WASM when the combined state changes to minimize overhead.
    - **Hot-plugging**: Detects connection/disconnection of gamepads.

### Memory Map
- **$0000-$00FF**: Zero Page.
    - `$E0`: Scroll X, `$E1`: Scroll Y.
    - `$E2`-$E3`: Random Seed (LFSR).
    - `$F8`: PPU Ctrl Shadow.
- **$0100-$01FF**: Stack.
- **$0200-$02FF**: OAM (Shadow Sprites).
- **$0300-$037F**: Sound Engine State (128 bytes).
    - Stride 32 bytes per channel (0, 32, 64, 96).
    - Offsets: State(0), Inst(4), Prio(5), Vol(6), Pitch(9), Arp(13), Base(17), Duty(19).
- **$0380-$03BF**: VBlank Buffer.
- **$03C0-$04BF**: String Heap (256 bytes).
- **$04C0-$07FF**: User Variables (DIM).
- **$8000-$CFFF**: PRG-ROM (Code).
- **$D000**: NTSC Period Table.
- **$D100**: Music Data.
- **$D480**: DPCM Sample Table (Addr, Len).
- **$D500**: Nametable Data (Ends at $D900).
- **$D900**: SFX Table.
- **$DA00**: Envelope Data Table.
- **$E000**: Palette Data.
- **$E040**: DPCM Samples (Start).
- **$FF00**: Data Tables (Vectors pointers).
- **$FFFA**: Vectors (NMI, Reset, IRQ).

- **Pitfalls**:
    - **WASM Pixel Format**: `tetanes-core` frame buffer format needs verification (RGBA vs RGB vs Palette). Currently assuming pointer access is sufficient for raw rendering, but color mapping might be needed if it returns raw NES palette indices. Future phases should verify color correctness.
    - `Text.Print` writes directly to the PPU ($2006/$2007). Use `WAIT_VBLANK` before calling this to avoid visual glitches.
    - `True` is `$FF`. Check assumptions in assembly injections if they rely on `1`.
    - **Audio Labels**: When injecting assembly strings in loops or multiple blocks, ensure labels are unique or use local labels if assembler supports it. `rs6502` global label reuse caused "Branch too far".
    - **DPCM Alignment**: Samples must start on 64-byte boundaries. Compiler handles padding.
    - **OAM Overflow**: `Sprite.Draw` drops sprites if 64 limit reached. Enable `Sprite.SetFlicker(1)` to mitigate limits via cycling.
    - **SFX Sequences**: `SequenceCanvas` modifies arrays in place.
    - **Frontend Validation**: When importing JSON, always validate fields exist to avoid `undefined` crashes in the editor.
    - **CHR Import**: Requires a 128x128 PNG for full bank import. Alpha channel is treated as color 0 (transparent). Nearest neighbor matching uses the *current* 4-color palette, not the full NES palette, so ensure the correct sub-palette is selected before importing.
    - **16-bit Pointers**: When calculating addresses (like Heap Offset), always handle 16-bit Carry (`BCC +; INX`) for the High Byte.
    - **Gamepad Polling**: `navigator.getGamepads` returns a snapshot. It must be polled in `requestAnimationFrame`.
    - **Input State**: When modifying input logic, ensure keyboard and gamepad don't conflict (e.g., releasing a button on one device shouldn't clear the hold on the other). Use a "last sent state" tracker.
    - **WASM Crash**: If the emulator crashes (e.g. `CpuCorrupted`), the loop stops and input polling ceases. Ensure robustness or handle errors gracefully if you need input to restart.

- **Next Steps**:
    - Start Phase 34: Debugging - Protocol.
    - Define shared memory or message passing interface (WASM <-> JS).
    - Expose CPU RAM and Registers to JS.

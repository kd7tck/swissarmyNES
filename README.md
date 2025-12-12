# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status
**Phase 1: Project Initialization** - The basic Rust backend structure is in place with a functional HTTP server.

## Features (Planned)
- **SwissBASIC**: A hybrid language combining BASIC syntax with inline 6502 Assembly.
- **Zero-Friction Toolchain**: Cloud-based compilation (or WASM).
- **Visual Editors**: Sprite, Map, and Palette editors.

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

3. **Verify:**
   Open your browser or use curl:
   ```bash
   curl http://localhost:3000
   ```
   You should see `Hello World`.

## License
[License Information]

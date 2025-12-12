# SwissArmyNES

**SwissArmyNES** is a comprehensive, web-based Integrated Development Environment (IDE) tailored for creating Nintendo Entertainment System (NES) games. It combines a Rust backend with a vector-based HTML/JS frontend to provide a modern workflow for retro game development.

## Current Status
**Phase 2: Frontend Foundation** - The backend serves static assets for the frontend. The frontend has a basic structure with navigation tabs (Code, Graphics, Settings) and a CSS framework focused on vector graphics.

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
   Open your browser to `http://localhost:3000`. You should see the SwissArmyNES IDE interface with tabs for Code, Graphics, and Settings.

## License
[License Information]

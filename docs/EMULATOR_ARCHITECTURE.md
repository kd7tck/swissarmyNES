# Emulator ownership and cartridge boundary

SwissArmyNES executes tetanes-core 0.12.2 in both native tests and the WASM build. The compiler targets mapper 1 with eight 16 KiB PRG banks. The dependency remains pinned by Cargo.lock; do not introduce a separate CPU solely for validation.

The project-owned `emulator/src/cartridge.rs` parser validates metadata and payload boundaries before backend allocation. `Emulator::load_rom` applies a narrow compatibility adapter: whole-bank exponent-size declarations are converted to equivalent linear counts, and a trainer is removed from the backend input and copied through the actual CPU bus to $7000-$71FF. The destination is verified before the replacement deck is accepted. Original caller bytes are never modified.

Header rules come from [NESdev NES 2.0](https://www.nesdev.org/wiki/NES_2.0) and [iNES](https://www.nesdev.org/wiki/INES). Both size encodings and independent volatile/nonvolatile RAM fields are decoded. Each ROM region is limited to 64 MiB before allocation on both native and WASM hosts. Truncated images, zero PRG, invalid signatures, unsupported header variants, and unsupported console/miscellaneous-ROM layouts fail before replacing a running game.

Debugger address identity comes from `MapRead::map_peek` on the production mapper. Physical PRG offsets exclude the header; debug breakpoints use 16 KiB bank units. Source-map `rom_offset`, in contrast, is a file offset including the header; it describes compiler-generated images without trainers. CPU peeks use the backend's side-effect-free Read::peek.

## Remaining phase-39 work

This is not complete architecture acceptance. In particular:

- The backend's default RAM allocation must be reconciled with explicit NES 2.0 absence and RAM/NVRAM declarations; metadata decoding alone does not prove allocation semantics.
- Partial-bank exponent layouts are rejected with a clear geometry error. Additional geometry support needs mapper-aware handling.
- Trainer persistence across hard reset needs an explicit policy and regression.
- Timing, reserved bits, input-device metadata, submapper behavior, and ambiguous legacy headers require further acceptance tests.
- The roadmap's named mapper operations and recording-mapper IRQ/bus integration remain to implement. Current routing still uses the upstream MapRead/MapWrite implementation.
- Canonical CPU trace comparison now passes all 8,991 records in native and generated browser WASM. Supplemental CPU/interrupt coverage is still incomplete.

Do not mark phase 39 or 40 complete based on the focused cartridge and debugger tests.

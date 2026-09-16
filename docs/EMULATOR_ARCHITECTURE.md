# Emulator ownership and cartridge boundary

SwissArmyNES executes tetanes-core 0.12.2 in both native tests and the WASM build. The compiler targets mapper 1 with eight 16 KiB PRG banks. The dependency is reconstructed by scripts/prepare-core.mjs from the checksum-pinned 0.12.2 archive plus patches/tetanes-core-0.12.2.patch, and Cargo.lock resolves its ignored local directory. Native and WASM use the same corrected CPU/core; do not introduce a separate CPU solely for validation.

The project-owned `emulator/src/cartridge.rs` parser validates metadata and payload boundaries before backend allocation. `Emulator::load_rom` applies a narrow compatibility adapter: whole-bank exponent-size declarations are converted to equivalent linear counts, and a trainer is removed from the backend input and copied through the actual CPU bus to $7000-$71FF. The destination is verified before the replacement deck is accepted. Original caller bytes are never modified.

Header rules come from [NESdev NES 2.0](https://www.nesdev.org/wiki/NES_2.0) and [iNES](https://www.nesdev.org/wiki/INES). Both size encodings and independent volatile/nonvolatile RAM fields are decoded. Each ROM region is limited to 64 MiB before allocation on both native and WASM hosts. Truncated images, zero PRG, invalid signatures, unsupported header variants, and unsupported console/miscellaneous-ROM layouts fail before replacing a running game.

Debugger address identity comes from `MapRead::map_peek` on the production mapper. Physical PRG offsets exclude the header; debug breakpoints use 16 KiB bank units. Source-map `rom_offset`, in contrast, is a file offset including the header; it describes compiler-generated images without trainers. CPU peeks use the backend's side-effect-free Read::peek.

Bus routing and mirroring (P39-06) is tested and verified in `emulator/tests/bus_test.rs`: 2KB internal RAM ($0000-$07FF) and its mirrors ($0800-$1FFF) alias the same underlying memory; PPU register mirrors ($2008-$3FFF) route properly; PRG-ROM writes are ignored (read-only); and peeking CPU state does not advance registers or alter PC/cycles.

## Remaining phase-39 work

This is not complete architecture acceptance. In particular:

- The backend's default RAM allocation must be reconciled with explicit NES 2.0 absence and RAM/NVRAM declarations; metadata decoding alone does not prove allocation semantics.
- Partial-bank exponent layouts are rejected with a clear geometry error. Additional geometry support needs mapper-aware handling.
- Hard reset is a fresh cartridge boot: original trainer bytes are reapplied through the CPU bus after RAM reset. A regression verifies all 512 bytes and subsequent ROM execution, including retention after a failed replacement load.
- Timing, reserved bits, input-device metadata, submapper behavior, and ambiguous legacy headers require further acceptance tests.
- The roadmap's named mapper operations and recording-mapper IRQ/bus integration remain to implement. Current routing still uses the upstream MapRead/MapWrite implementation.
- Canonical CPU trace comparison now passes all 8,991 records in native and generated browser WASM. Supplemental CPU/interrupt coverage is still incomplete.

Do not mark phase 39 or 40 complete based on the focused cartridge and debugger tests.


Compiler mapper writes: generated PRG switching uses 07F0 as desired bank and 07F2 as interrupt retry flag. Each attempt resets the serial register and writes all five PRG bits. NMI/IRQ epilogues flag interrupted attempts for retry; wrappers preserve 07F1 return scratch. This protocol assumes fixed-upper-bank mode and generated writes. Arbitrary inline assembly writes need separate coordination.


Dynamic interrupt binding uses one-byte selectors at 07F8 (NMI) and 07F9 (IRQ). Zero keeps the startup pointer. Nonzero indexes resolve immutable fixed-bank low/high ROM address tables; banked SUB handlers use generated trampolines. ON accepts zero-argument routines and publishes its selector in one store, avoiding torn pointers when interrupted or when rebinding from a handler.


Interrupt temporary storage now comprises 00-0F, 14-17 and F0-F7, saved through a sparse ROM address table, plus 07F1 and CPU registers. Persistent controller/text/sprite/scroll/RNG state is deliberately not rolled back. Idle audio channels are skipped during Sound_Update. An idle NMI including DMA measures 1774 CPU cycles in native execution; active workloads still need a vblank budget and fixed-bank code space remains tight.

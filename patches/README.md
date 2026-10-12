# TetaNES core source patch

Run `node scripts/prepare-core.mjs` before building a fresh checkout. It verifies
the upstream tetanes-core 0.12.2 package SHA-256, extracts it into ignored build
storage, checks and applies the adjacent source patch, then replaces the generated
dependency directory. The archive retains upstream authorship and MIT/Apache-2.0
license metadata. No upstream binaries or generated files are committed here.

The patch corrects independent NES 2.0 volatile/nonvolatile RAM size decoding,
NROM's explicit PRG RAM absence/small-capacity mirroring, and MMC3 four-screen
nametable mirrors. The MMC3 correction keeps $3000-$3EFF accesses inside its
4 KiB extra RAM; a production CPU/PPUDATA regression reproduced the prior
out-of-bounds panic. Legacy NROM keeps its default RAM behavior. Native tests
and generated WASM resolve the same patch through the root Cargo declaration.

RAM and NVRAM share backend storage, with declared battery partitions owned by
the emulator wrapper's snapshot/restore/reset policy. Mapper allocations that
disagree with NES 2.0 declarations are rejected before replacing the active game.
Unsupported NROM geometries and board capabilities are reported explicitly.
See [runtime contracts](../docs/DESIGN.md#runtime-contracts) for the supported boundary;
this patch does not imply unrestricted NES 2.0 board support.

When editing the patch, regenerate the dependency, run the native cartridge and
CPU regressions, rebuild WASM, and run `node --test tests/js/*.test.cjs`.

[Setup/verifier](../README.md#verification) pins Rust 1.98.0 and wasm-bindgen
0.2.106. Prepare the core only while Cargo is idle. An abandoned `.tools/core-prepare-*`
directory can be removed after confirming the active `.tools/tetanes-core` is intact;
do not delete the active dependency or installed build tools during temporary cleanup.

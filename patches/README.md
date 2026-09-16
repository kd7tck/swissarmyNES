# TetaNES core source patch

Run `node scripts/prepare-core.mjs` before building a fresh checkout. It verifies
the upstream tetanes-core 0.12.2 package SHA-256, extracts it into ignored build
storage, checks and applies the adjacent source patch, then replaces the generated
dependency directory. The archive retains upstream authorship and MIT/Apache-2.0
license metadata. No upstream binaries or generated files are committed here.

The patch currently corrects independent NES 2.0 volatile/nonvolatile RAM size
decoding and NROM's explicit PRG RAM absence/small-capacity mirroring. Legacy NROM
keeps its previous default RAM behavior. The production emulator and native tests
resolve the same patched dependency through the root Cargo patch declaration.

RAM and NVRAM capacity is combined in the backend bus; separate nonvolatile save
and reset policies remain unfinished. Other mappers' implicit allocations have
not yet been reconciled. Do not interpret this patch as complete NES 2.0 support.

When editing the patch, regenerate the dependency, run the native cartridge and
CPU regressions, rebuild WASM, and run `node --test tests/js/*.test.cjs`.

# Baseline audit for the phase 40 developer handoff

Repository: <https://github.com/kd7tck/swissarmyNES>

Current HEAD examined: `355cb1e` (2026-09-17) — Supersedes commit `4610516` (2026-09-12).

## Re-baseline Status at HEAD (355cb1e)

The audit issues previously blocking `POST /api/compile` on minimal programs are closed at HEAD:
- Minimal `SUB Main()\nEND SUB`: Returns HTTP 200 (valid 139,280-byte MMC1 ROM).
- WORD math `LET w = 1000 + 500`: Returns HTTP 200 (correct 1500 result at `$05C0` in RAM).
- Controller call `Controller.Read()`: Returns HTTP 200 (emits runtime helpers in switchable bank).

### Audit Item Resolution Summary

- **Closed at HEAD**: A01 (compile_source invokes generate_banks), A02 (helper availability in banked path), A03 (current_prg_bank defined), A04 (RAM allocation bounds error past `$07F0`), A05 (versioned LinkedSourceMap contract), A22 (3-OS CI matrix).
- **Open at HEAD**: A23 (empty project/file name validation).
- **New Findings Identified**:
  - **F1**: Parser/analyzer/codegen recursion depth guard required to prevent process SIGABRT on deeply nested inputs.
  - **F2**: Tracked WASM and binary test artifacts in git despite `.gitignore`.
  - **F3/F4**: MMC1 default ROM size and mirroring configuration.
  - **F5**: Constant division/modulo by zero compile-time diagnostic.
  - **SUB parameters**: Scope retention issue during codegen.
  - **FOR loop step**: Unsigned underflow on `FOR i = 10 TO 0 STEP -1` with `BYTE`/`WORD`.
  - **SELECT CASE**: Support for `TO` ranges and `IS` comparisons in parser/codegen.

---

Historical record below reflects initial audit state at commit `4610516840d7991bce7e1d879e1f4e2fa831f10e`:

## Executed checks

| Check | Result | Limits |
|---|---|---|
| Clone and inspect Git state | Successful clone; original tree clean | Does not include later upstream commits |
| `rustc --version` | `rustc 1.98.0 (88d9e12ae 2026-08-18)` | Windows MSVC environment |
| `cargo --version` | `cargo 1.98.0 (797e8a9bc 2026-08-05)` | Same host |
| `rustup target list --installed` | `x86_64-pc-windows-msvc` | WASM target was not installed during audit |
| `cargo test --workspace --all-targets --locked` | Exit 0; all executed tests passed | Emulator crate ran zero tests; many compiler tests inspect assembly text |
| `cargo fmt --all -- --check` | Exit 0 | Formatting only |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0 | Native checks, not browser functionality |
| `cargo run --locked` | Server started; compile API reachable at `127.0.0.1:3000` | Manual browser interaction not performed |
| `POST /api/compile`, empty Main | HTTP 400 | Exact error below |
| `POST /api/compile`, WORD addition | HTTP 400 | Exact error below |
| `POST /api/compile`, controller call | HTTP 400 | Exact error below |

The initial dependency fetch failed under restricted network access. It was rerun with approved network access, dependencies were downloaded from the lockfile, and the workspace test completed successfully. This was an environment access issue, not a repository test failure.

## Compile API reproductions

Run the server with `cargo run --locked`. Send JSON with a `source` field and `Content-Type: application/json` to `http://127.0.0.1:3000/api/compile`.

### Minimal

```json
{"source":"SUB Main()\nEND SUB"}
```

Observed response body:

```text
Assembler Error: "Assembler error (Banks 0 & 7): AssemblerError { message: \"Invalid addressing mode for opcode. Line 314\" }"
```

### WORD math

```json
{"source":"DIM w AS WORD\nSUB Main()\n LET w = 1000 + 500\nEND SUB"}
```

Observed response body:

```text
Assembler Error: "Assembler error (Banks 0 & 7): AssemblerError { message: \"Invalid addressing mode for opcode. Line 336\" }"
```

### Controller

```json
{"source":"SUB Main()\n Controller.Read()\nEND SUB"}
```

Observed response body:

```text
Assembler Error: "Assembler error (Banks 0 & 7): AssemblerError { message: \"Invalid addressing mode for opcode. Line 315\" }"
```

Cross-platform API tests should preserve these source inputs, but assert successful compile/boot after repair. Do not bind the final regression test to an assembler line number that will change during repair.

## Source findings and repair mapping

Line references describe the audited commit and may move during implementation. “Observed in source” is distinct from “reproduced at runtime.”

| ID | Evidence | Interpretation / next action |
|---|---|---|
| A01 | `src/server/api.rs::compile_source` invokes `generate_banks`; many tests, including `tests/e2e_test.rs`, call `generate` | Verified route mismatch; R02 adds API-path tests |
| A02 | `src/compiler/codegen.rs` around lines 64–123 versus 227–259 | Banked path omits controller/text/sprite/animation/pool/collision/scroll/user-data helpers emitted by legacy path; repair shared generation |
| A03 | `codegen.rs` lines 102 and 629 reference `current_prg_bank`; repository search found no definition | Undefined-symbol risk in assembly; inspect exact generated failing line before assigning root cause |
| A04 | `codegen.rs::allocate_memory` allows allocation through `$07FF`; debugger reads `$07F0` | Bank-shadow reservation absent from documented allocator limits; add a reserved-region policy if retaining the shadow |
| A05 | `codegen.rs` line 18 `SourceMap = Vec<(usize,u16)>`; `editor.js` hover/sync loops destructure triples, active-line loop pairs | Confirmed incompatible contracts; R04 is required |
| A06 | `codegen.rs::estimate_size` tests operand prefixes before DB/WORD; accumulator operands fall through; some instructions use direct `output.push` | Multiple source-map drift risks; add byte-level mapping tests |
| A07 | `assembler.rs::assemble_banks` allocates 131,072 PRG bytes and emits header `[...08,01,10,00...]` | Current compiler output is mapper 1, 128 KiB PRG + 8 KiB CHR, 139,280 total bytes |
| A08 | Segment length checks use complete PRG vector limits | A segment can cross its CPU bank window without exceeding the complete vector; add per-bank checks |
| A09 | `allocate_memory` stores sub signatures using `self.current_bank` without processing BANK in that pass | Source-derived bank-signature risk; test multi-bank declarations and calls |
| A10 | `generate_trampolines` resolves symbols across banks; assembler combines each bank with fixed-bank assembly separately | Cross-bank symbol resolution/return behavior needs tests; not proven by existing tests |
| A11 | `api.rs` nametable injection uses `nametables.first()`; world data stores map indices | ROM integration of other world maps is not established; phase 29 repair |
| A12 | `SoundEffect` contains volume/pitch/duty sequences, no explicit arpeggio field | Check whether existing pitch semantics satisfy SFX arpeggios; if absent, implement end to end |
| A13 | Audio schema comment says channel 3 Noise; frontend has DMC comments for track 4 | Channel mapping must be verified and documented consistently |
| A14 | `Emulator::step` executes an instruction before its first breakpoint check and uses guest RAM `$07F0` for bank identity | Initial breakpoint and arbitrary-ROM/fixed-bank identity risks; phases 34/36 |
| A15 | `editor.js::emulatorLoop` advances one emulated frame per animation callback | Refresh-rate-dependent speed risk; phase 32 pacing tests |
| A16 | `ppu_viewer.js` constructor sets display none, then flex | Source-confirmed initial visibility inconsistency |
| A17 | TetaNES `ppu.rs::load_palettes` writes 16×2 RGBA; wrapper allocates 4 KiB; frontend reshapes all returned bytes as one row | Confirmed buffer-contract mismatch; phase 38 |
| A18 | `static/wasm` contains generated JS/declarations plus two WASM binaries; `emulator/wasm-pack` starts with ELF magic; stamp says 0.13.1 | Build artifacts exist, but reproducibility/currentness not established; R01 |
| A19 | TetaNES 0.12.2 `cart.rs::NesHeader::load` explicitly rejects trainer flag | Source-confirmed parser limitation relevant to phase 39 |
| A20 | Same parser combines NES 2.0 size high nibbles into bank counts | Exponent/multiplier support not established; add failing fixtures before patching |
| A21 | TetaNES `mapper.rs` uses MapRead/MapWrite and mapper enum/event hooks | Current architecture differs from roadmap's named Mapper operations; actual execution bridge required |
| A22 | CI uses ubuntu-latest and root `cargo build/test/clippy/fmt` commands | Add host matrix and explicit workspace/WASM/browser coverage |
| A23 | Project/file validation checks characters but does not reject empty names | Add meaningful-name validation and integration tests in phase 4 |
| A24 | README claims phases 1–30; Brain claims 31–38 and points to old mapper phase; DESIGN changed 39/40 | Historical statuses are not current acceptance evidence |

## Dependency inspection

Read locked `tetanes-core` version **0.12.2** from Cargo's downloaded source, especially:

- `src/control_deck.rs`: ROM loading, instruction/frame stepping, CPU/PPU access, WRAM and frame buffers.
- `src/cart.rs`: iNES/NES 2.0 header loading and trainer rejection.
- `src/bus.rs`: CPU access routing and existing internal bus tests.
- `src/mapper.rs`: dispatch and mapper event contracts.
- `src/cpu.rs`: CPU state, trace, interrupt/DMA helpers.
- `src/ppu.rs`: pattern/nametable/palette export buffer contracts.
- `test_roms/cpu/nestest.txt`: dependency trace formatting differs from the usual canonical log.

This inspection did not run TetaNES's own internal test suite or canonical nestest. A workspace dependency's internal tests are not automatically run by this application's `cargo test --workspace`.

## Not performed / not established

- No application bug fixes, new CPU, cartridge parser, mapper bridge, or phase implementations were made.
- No WASM target installation, fresh WASM generation, visual browser run, audio listening, physical gamepad test, or native macOS/Linux test was performed.
- No canonical ROM/log pair was downloaded and hashed for this repository, and no nestest success is claimed.
- Existing generated WASM bytes were not proven to match the current Rust source.
- Source-derived gaps beyond the three API failures were not all separately reproduced in execution.
- No exhaustive security, hardware-conformance, or performance audit is implied.

These are explicit work items in the handoff, not reasons to mark phases complete. The report and plan are complete planning artifacts; the implementation acceptance remains pending.

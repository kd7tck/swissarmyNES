# Phase 40 acceptance worksheet

Use with [the developer handoff](DEVELOPER_HANDOFF_THROUGH_PHASE_40.md) and [baseline audit](BASELINE_AUDIT.md).

All items start pending. Historical roadmap completion labels do not check these boxes. For each item, add a completion commit/PR, automated command and result, fixture/evidence path, and reviewer/date. A proposed task is not an audit pass.

## Foundation gates

- [x] M0: baseline recaptured against the developer's actual HEAD (355cb1e); current roadmap numbering reconciled.
- [ ] R01: native host matrix, workspace checks, pinned WASM generation, browser harness, clean setup.
- [ ] R02: previously failing API fixtures now compile and boot; production/legacy helpers unified.
- [ ] R03: bank windows, reserved RAM, asset lengths, overflow, vectors, and deterministic output verified.
- [ ] R04: versioned file/bank/range source map and all consumers agree with actual assembly.
- [ ] R05: headless and browser harnesses execute the production backend with bounded runs.

## Per-phase acceptance

| Complete | Phase | Required proof | Commit / command / evidence / reviewer |
|---|---|---|---|
| [ ] | 1 | Clean checkout setup and native/WASM quality gates | Pending |
| [ ] | 2 | Initialized strings, LEN and heap boundaries execute correctly | Pending |
| [ ] | 3 | DATA/READ values and RESTORE positions correct in RAM | Pending |
| [ ] | 4 | Included subroutine runs; multi-file persistence and diagnostics correct | Pending |
| [ ] | 5 | WORD arithmetic produces 1500 and boundary cases | Pending |
| [ ] | 6 | Multiply/divide/signed comparisons and edge cases | Pending |
| [ ] | 7 | SELECT branches/default/nesting match intended behavior | Pending |
| [ ] | 8 | Struct/array field offsets and allocation limits | Pending |
| [ ] | 9 | Enum values and constant-equivalent behavior | Pending |
| [ ] | 10 | Macro expansion, provenance, recursion rejection | Pending |
| [ ] | 11 | Controller pressed/held/released state transitions | Pending |
| [ ] | 12 | Text at (10,10), font mapping and safe PPU writes | Pending |
| [ ] | 13 | Eight-tile metasprite and documented overflow/flicker behavior | Pending |
| [ ] | 14 | Three-frame animation duration/loop/restart | Pending |
| [ ] | 15 | Ten entity slots, full failure, despawn/reuse | Pending |
| [ ] | 16 | AABB overlap, edges, range limits | Pending |
| [ ] | 17 | Point/tile lookup and actual solid-tile movement stop | Pending |
| [ ] | 18 | Horizontal seams/attributes and bounded upload buffers | Pending |
| [ ] | 19 | Vertical seams/attributes and direction changes | Pending |
| [ ] | 20 | Seed repeatability and RND range/variation | Pending |
| [ ] | 21 | WAV import → DPCM data → button-triggered sound | Pending |
| [ ] | 22 | Lower/equal/higher priority behavior and music resumption | Pending |
| [ ] | 23 | Drawn envelopes persist and reproduce in playback | Pending |
| [ ] | 24 | 0,4,7 arpeggio pitch sequence and bounds | Pending |
| [ ] | 25a | Defined SFX binary and runtime sequence behavior | Pending |
| [ ] | 25b | SFX list/properties persist through reload | Pending |
| [ ] | 25c | Volume/pitch/duty/arpeggio visual editing works end to end | Pending |
| [ ] | 25d | JSON export/drop/import → compile → sound | Pending |
| [ ] | 26 | PNG conversion produces expected CHR and palette result | Pending |
| [ ] | 27 | 16×16 metatile order/palette/persistence | Pending |
| [ ] | 28 | Individual-tile/metatile edits remain consistent | Pending |
| [ ] | 29 | All referenced world map bytes exist in ROM and load correctly | Pending |
| [ ] | 30 | UI-created character/animation usable in code | Pending |
| [ ] | 31 | Fresh WASM builds; Run loads current ROM with correct pixels | Pending |
| [ ] | 32 | Controls, audio, lifecycle, refresh-independent pacing | Pending |
| [ ] | 33 | Keyboard + physical gamepad, mixed holds, disconnect recovery | Pending |
| [ ] | 34 | CPU/RAM snapshots, peeks, and mapper-derived address identity | Pending |
| [ ] | 35 | Actual ROM mapping, included files, exported sourcemap.json | Pending |
| [ ] | 36 | Stop-before-execute, initial breakpoint, continue and re-hit | Pending |
| [ ] | 37 | Full internal RAM display, live/paused refresh, bounded rendering | Pending |
| [ ] | 38 | Pattern/nametable/palette/OAM views match actual mapped state | Pending |
| [ ] | 39 | All architecture/parser/bus/mapper tasks below | Pending |
| [ ] | 40 | All CPU/golden-log tasks below | Pending |

## Phase 39 detailed gate

- [ ] P39-01: actual core ownership and maintained patch strategy documented.
- [ ] P39-02: normalized metadata and parse/unsupported errors tested.
- [ ] P39-03: iNES sizes, trainer, CHR RAM, mirroring, malformed/truncated input.
- [ ] P39-04: NES 2.0 linear/exponent sizes, extended mapper/submapper, RAM/NVRAM.
- [ ] P39-05: named Mapper operations connected to actual CPU/PPU dispatch, proven by integrated routing tests.
- [x] P39-06: complete 16-bit bus boundary/mirror/side-effect/peek tests.
- [ ] P39-07: NROM fixtures, existing mapper-1 games, transactional load/reset, native/WASM acceptance.

## Phase 40 detailed gate

- [ ] P40-01: deterministic, side-effect-free pre-instruction tracing from production backend.
- [ ] P40-02: 256-entry classification; 151 official encodings and 56 mnemonics covered.
- [ ] P40-03: every addressing form, wrapping, page penalties, dummy accesses, cycle totals.
- [ ] P40-04: ALU flags, binary arithmetic with D set, stack order/wrapping.
- [ ] P40-05: RESET/NMI/IRQ/BRK/RTI, masking/priority/polling, interrupt and DMA timing.
- [ ] P40-06: required unofficial encodings and safe JAM/reset behavior.
- [ ] P40-07: fixture source revisions, SHA-256, record count, initial/terminal state recorded.
- [x] P40-08: complete canonical log matches PC/A/X/Y/P/SP/cycles exactly; comparator negative tests pass.
- [ ] P40-09: any CPU corrections include isolated regressions and are used by both native and browser builds.
- [ ] P40-10: supplemental CPU tests, host matrix, WASM equivalence, compiler/game regressions pass.

## Final release gate

- [ ] Integrated authored demonstration in handoff section 11 completed.
- [ ] Clean checkout reproduces all published commands and required generated artifacts.
- [ ] No required phase feature is merely deferred to a later-phase backlog.
- [ ] README, DESIGN, AGENTS Brain, architecture, schemas, support matrix, coverage, and fixture notices updated.
- [ ] Evidence artifacts identify the same implementation commit and core revision.
- [ ] Windows, macOS, Linux, browser, audio, and physical controller evidence attached.
- [ ] Reviewer signs off that phases 1–40 (including 25a–25d) meet their criteria.

Implementation commit: **Pending**

Core version / patch revision: **Pending**

Canonical ROM SHA-256 / log SHA-256 / records compared: **Pending**

CI run and browser evidence: **Pending**

Reviewer / date: **Pending**

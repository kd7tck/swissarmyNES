# Runtime contracts and verification

The implementation uses the same checksum-pinned TetaNES 0.12.2 source and project patch in native and generated WASM builds. Run `node scripts/verify.mjs` for the complete local matrix. The native CI job runs Windows, macOS, and Linux; a configured matrix is not evidence that a remote run has passed.

## CPU/debugger interface

`trace_instruction()` returns a pre-instruction snapshot and executes one production-core instruction. `peek_cpu()` and RAM/PPU debug inspection do not acknowledge hardware registers. Physical PRG identity comes from the executing mapper, in 16 KiB units. `CpuState.cycles` is a `u64`, exported as JavaScript `BigInt`; never coerce it to a 32-bit integer. Free wasm-bindgen snapshot objects after copying their fields.

`viewport_position()` returns `[x,y]` in the logical 512×480 nametable space. It decodes the PPU's programmed temporary address and fine X without reading status or changing the write latch. It describes the current programmed origin, not a raster-effects history.

`emulator/tests/cpu_contract.rs` inventories all 256 dispatch entries and the 151 official encodings/56 mnemonics. The canonical trace exercises 149 official encodings; BRK and CLI have separate execution tests. The suite exhaustively verifies ADC/SBC over all byte operand pairs, carry inputs, and D states, plus page penalties, zero-page wrapping and all 12 JAM error/reset paths. Separate interrupt/trace tests cover stack-status serialization, IRQ masking/polling, NMI, BRK/RTI, and OAM DMA. Supplemental timing evidence is recorded in the acceptance worksheet; do not infer every bus timing edge from opcode coverage alone.

## Cartridge battery layout

The loader commits a replacement only after metadata, backend allocation, and trainer validation succeed. NES 2.0 allocations that differ from declared capacity are rejected explicitly. Unsupported combinations of CHR ROM with nonvolatile CHR RAM are rejected; partial-bank exponent geometries remain unsupported.

Volatile PRG RAM precedes PRG NVRAM in the backend allocation; volatile CHR RAM precedes CHR NVRAM. `save_ram_snapshot()` returns **PRG NVRAM followed by CHR NVRAM**, excluding volatile bytes. `restore_save_ram(bytes)` validates the complete payload length before writing either region. The caller owns storage and association with the right ROM; equal size alone does not identify a cartridge. A hard reset reinitializes volatile state, restores declared battery regions, and reapplies original trainer boot data. Loading another ROM starts another battery owner; a failed load retains the current owner. Legacy iNES battery headers use their normalized PRG battery capacity.

## Fixed-bank helper placement

The compiler retains runtime instructions reachable from startup, guest code in every bank, interrupts, symbolic inline-assembly operands, and transitive helper references. Literal data survives pruning. The assembler then resolves final addresses and checks segment overlap. Helper CPU addresses are not a stable public ABI; use named labels for inline assembly. This leaves space before fixed audio/asset segments without silently relocating or overwriting them.

Dynamic assignment uses the destination type's storage width. Assigning a WORD expression to a BYTE array writes one byte; assigning BYTE to WORD initializes the high byte. `STR` renders WORD unsigned and INT/negated expressions signed; `VAL` and `STR` preserve their cursors across arithmetic helpers.

## Sound and SFX data

Use `Sound.Play(track_id)` for music, `PLAY_SFX effect_id` for effects, and `Sound.Stop()` for hardware/software silence. Invalid or missing runtime IDs leave an active channel alone. Track channels are 0=Pulse1, 1=Pulse2, 2=Triangle, 3=DMC. Macro SFX use tonal channels 0–2. Lower priority cannot replace higher priority; equal priority retriggers/replaces. An effect pauses the displaced music timer and restores its note/envelopes on completion.

Older SFX JSON loads without `arpeggio_sequence`, defaulting to `[]`. Effects contain name/channel/priority/speed/does_loop plus volume (`u8`), pitch (`i8`), duty (`u8`), and arpeggio (`i8`) sequences. Volume is 0–15, duty 0–3; each sequence has at most 127 steps. Pitch is an independent timer offset; arpeggio is signed semitones from macro base **note 48**. The historical 96-entry period table is preserved; its old octave labels must not be used as a physical-frequency guarantee.

Music at `$D100` and effects at `$D900` start with a count and little-endian absolute pointers. An effect record is six bytes: channel, priority, volume-envelope ID, pitch-envelope ID, duty-envelope ID, arpeggio-envelope ID. IDs are assigned after user envelopes. Envelopes at `$DA00` have count/pointers, then a loop index (`$FF` for no loop), `[value,duration]` steps and a zero-duration terminator. Speed zero means one frame. Finite sequences hold their final values until the longest sequence completes; each loop wraps independently. Music notes are `[duration,pitch]`, with pitch `$FF` as rest and duration zero terminating the track. Empty tracks terminate immediately.

DPCM tables at `$D480` store address/length pairs. Sample data begins at `$E040`, is 64-byte aligned, and is padded to `16*n+1` bytes. Individual samples support at most 4081 bytes and are rejected rather than truncated. A DMC track selects the authored sample and rate, independently of the tonal period table. Count and segment capacity checks apply before injection.

Music-editor Play compiles the selected track through the production APU runtime. Notes serialize as numeric values; DMC rows select sample IDs with 16 rate choices. WAV import resamples to the selected NTSC DMC bit clock, rejects oversized/invalid input, and preserves the current project if an asynchronous import becomes stale. Removing a sample or envelope repairs remaining track references. The tracker displays the physical octave of the preserved timer table: index 48 is C6.

## World maps, uploads and gameplay

`World.Load(screenColumn, screenRow)` returns `$FF` on success and zero for an empty/out-of-bounds cell. It loads all 960 tiles and 64 attributes with rendering disabled, then restores mapper bank, PPU control/mask, fine scroll and caller IRQ mask. A failed lookup preserves the selected map. `World.Tile(localPixelX, localPixelY)` and `Collision.Tile` read that selected map and reject coordinates outside 256×240. Legacy one-map projects retain the `$D500` image.

World assets use linker-checked data in unused MMC1 banks 1–6. Directory version 1 starts with `[version,width,height,stride=5]`; each cell has `[bank,addressLow,addressHigh,lengthLow,lengthHigh]`, with bank `$FF` for empty and length 1024 for a map. Maps are 1024-byte aligned and deduplicated by authored map index. The directory supports at most 3276 cells; actual map capacity depends on free banks and directory padding. RAM `$07FA-$07FC` stores the selected map; zero-page `$F9` shadows PPUMASK.

`Scroll.LoadRow(y, tiles, optionalAttributes)` and `LoadColumn(x, tiles, optionalAttributes)` accept BYTE arrays with at least 32/30 tiles and, when supplied, 8 fully composed attribute bytes. The queue uses at most 46 bytes within `$0380-$03BF`: four header bytes, tile data at offset 4, attribute address at 36 and attribute data at 38. Tile and attribute phases run on consecutive NMIs; busy requests are skipped. Wait until `PEEK($0380)=0` before exposing a new seam. `$FD` advances after the generated NMI audio update and is the completed-frame signal used by WAIT_VBLANK when NMI is enabled.

Built-in PPU work precedes audio so active music/effects do not delay VRAM/scroll writes past vblank. Pool allocation reports its full sentinel, clears reused slots, and validates despawn indices without writing neighboring RAM. The [editable platform demo](../examples/platform-demo/README.md) exercises maps, includes, graphics, animation, input, wall collision, audio and pool reuse together.

## Frontend ownership

Studio generates protected ordinary source from validated metadata; preview/tests compile an owned design/assets snapshot in the same production core and preserve custom main. Scene/entity/event schema, RAM/OAM limits, deferred entry/visit semantics and variable assertions are documented once in [Studio contracts](GAME_STUDIO.md). The bounded harness excludes custom-main/device/fidelity acceptance. [Checkpoint/evidence](IMPLEMENTATION_RESUME.md).

First Run/music preview shares one WASM initialization. Constructor and memory exports publish together only after validation; failed initialization permits a later retry. Only the latest Run for the same source/project/file context may compile. Closing the emulator or reloading its project invalidates pending startup/compile results and suppresses obsolete errors. Static IDE assets use `Cache-Control: no-cache`, including conditional responses, so unversioned JS/WASM is revalidated on subsequent requests. The PPU debugger is layered above the emulator overlay.

Full scaling fits the canvas into the remaining overlay height after the toolbar, including wrapped controls. The 1x and 2x modes retain intrinsic 256×240 and 512×480 content sizes. Full preserves the displayed image's aspect ratio and keeps Pause/Reset/Close within the viewport.

Project saves serialize and retain source/assets/project identity for each request; failed saves abort compilation. A later edit or project switch invalidates pending compile responses. One keyboard binding owns an emulator session; keyboard and gamepad holds combine, and both Shift keys can hold Select independently. Pause/reset/reload cancels queued audio and rebases scheduling. Hidden/closed sessions release inputs. Source maps preserve file/bank identity and reject edited snapshots. PPU views validate lengths and recreate memory views after growth.

The automated frontend harness verifies these workflows and graphics/SFX authoring round trips. Actual browser rendering, audible output, and physical controller observations require separately recorded evidence.

# GitHub integration in progress — October 10, 2026

User authorized committing/pushing all saved source changes, consolidating the matching GitHub integration branch, and merging PR #162 only after all GitHub checks pass. This supersedes prior pause/no-commit instructions. Development branch: `codex/nes-platform-completion-20261010`; GitHub main fetched at `355cb1e`, integration branch at `234944f`.

Windows browser authentication is complete through GCM and GitHub CLI as kd7tck. Full native workspace/all-target tests and strict Clippy passed locally after fixing byte/signed Clamp code generation and deterministic/completed-NMI audio regression observation. Fresh WASM and the complete Node matrix are in progress; logs and per-file recovery state are under ignored `.tools/notes/github-merge-2026-10-10.md` and `.tools/github-merge-*.log`.

Next: finish current local checks, verify source-only staging, commit saved work, reconcile the remote branch while retaining its meaningful additions, rerun affected/integrated checks, push to the existing PR branch, wait for all native host/WASM GitHub checks, merge, and verify the final main run. Actual browser presentation/audio/physical controller signoff remains separate from CI; do not claim full phase acceptance from the merge alone.

# Credit-low checkpoint — October 9, 2026 (World/demo/core repair saved)

**User requested another checkpoint and pause. Stop implementation; no commit has been made.** The work remains on `main` based on `bd42af4`, with all modified/untracked source preserved. The continuation below is historical until the user resumes.

## Latest completed work and evidence

- **World workflow is implemented:** complete linker-managed map/attribute images in unused MMC1 banks, `World.Load`, `World.Tile`, selected-map `Collision.Tile`, map selection/deletion/reindexing, and coordinate-preserving world resizing. **6/6 production World tests and 38/38 focused frontend tests passed.** Directory/capacity, four-map readback, backtracking, empty/OOB cells, bank/control/scroll/IRQ restoration and 20-map cross-bank storage are covered. Worker notes: `.tools/notes/frontend-acceptance-2026-10-09.md`.
- **Game timing and pool fixes pass targeted tests.** Built-in PPU writes under three looping tonal effects plus DMC finish at **964–1324 cycles**, below 2273-cycle NTSC vblank; full NMIs take **4294–5204 cycles**, below a frame. User NMI handlers remain responsible for their own PPU budget. Solid-wall movement and ten-slot capacity/cleared-reuse/invalid-despawn/neighbor preservation pass.
- **Editable platform demo is installed** at ignored `projects/platform-demo`. Source is `examples/platform-demo/{main.swiss,rooms.swiss,assets.mjs,install.mjs,README.md}`; installer preserves existing user data. Production test passes includes/source maps, ≥3 rendered colors, eight OAM tiles, animation, input press edges/SFX, wall stopping, room switching, and pool behavior. Test: `tests/platform_demo_execution_test.rs`.
- **Emulator boundary/core audit:** 31 targeted tests passed after `node scripts/prepare-core.mjs` successfully applied the full verified patch. Includes complete failed-load IRQ/NMI/DMA preservation, capability/header checks, scheduled NMI hijacking/reset isolation, actual MMC3 IRQ routing and four-screen mirror behavior. `patches/tetanes-core-0.12.2.patch` now appends two MMC3 nametable-mask corrections (`1FFF` → `0FFF`), preserving existing Cart/NROM sections. The real PPUDATA mirror test reproduced an ExRAM index4096/length4096 panic before repair and passes afterward. Simultaneous independent emulator instances remain unsupported by the thread-local backend signals; browser owns one active instance.
- **Root music/DPCM authoring fixes:** Play now compiles selected music through the production APU/emulator. Notes serialize numeric pitch/column0 correctly; DMC grid selects sample IDs and all 16 rates; WAV import resamples to the selected DMC clock, rejects >4081 bytes and closes its context; deleting samples/envelopes repairs references. Tracker physical note labels reflect the preserved period table. **7/7 new `audio_authoring.test.cjs` tests passed.** The final review was interrupted at the pause; no further review edits were made.
- **Earlier generated-WASM matrix passed 39/39**, including canonical trace, BigInt cycles, ALU/JAM and battery checks. **That bundle predates the latest emulator/core patch and music/World authoring changes; it is not final current-head evidence.** Generated bindings remain ignored.

## First actions on resume

1. Preserve the tree, then fix strict Clippy's known error at `src/compiler/codegen.rs:308`: `line.trim().split_whitespace()` must use `line.split_whitespace()`. Latest Clippy exited101; log `.tools/completion-clippy-final-20261009.log`. Do not suppress the lint. Formatting ran before that Clippy job, but final formatting/checks must be repeated after all subsequent edits.
2. Run targeted World/demo/game/NMI/audio/emulator suites, full workspace tests, formatting and strict Clippy. Last full-native log `.tools/completion-native-resume-20261009.log` failed the pool-tail assertion **before its fix**; the worker's corrected targeted pool test passes, but full native was not rerun. No build/test/server jobs remain running.
3. Rebuild release WASM against the newly prepared core, regenerate with wasm-bindgen0.2.106, run **all** `tests/js/*.test.cjs` (now includes the seven music authoring tests and expanded authoring tests). Keep BigInt cycle assertions. Use `scripts/verify.mjs`; do not re-prepare the core during a build.
4. Finish actual browser verification against a freshly built server. Browser helper initialized in CUA, but every local-page attempt failed before the IDE loaded. Sandboxed local socket access is forbidden (`Invoke-WebRequest` reported access permissions); the copied server initially died with its launching command, then foreground PTY server session81142 listened but was stopped cleanly before core preparation. **No actual browser/audio/controller acceptance was performed.** Rebuild the server and, if sandbox prevents access again, request the required `exec_command` network-sandbox escalation for its local launch; no escalation was requested yet. Agent tabs were only connection-error pages, not usable IDE sessions.
5. Re-run/review the root music-preview/bootstrap changes, verify demo save/compile/run, visual rendering and breakpoints/downloads. Demo native probe addresses: xWORD05C0, yWORD05C2, room05C4, loaded05C5, sound_hits05C6, frameWORD05C7, blocked05C9; eight sprites. Initial32/128/room0; Right200frames stopsx160; Up40 thenRight100 entersroom1; Apress increments sound_hits once per edge. Installer command: `node examples/platform-demo/install.mjs`.
6. Update the stale shared docs: World/scroll/pool/NMI contracts, supported cartridge layouts, patch README's new MMC3 repair and implemented battery policy, completion evidence and actual release limitations. The acceptance worksheet still has historical pending entries; fill only from actual results, not configuration/mocks. Run source-only staged-tree checks and commit when this integrated batch is verified. Generated `.tools`, `target`, WASM, downloaded ROMs and logs stay excluded.

Recovery notes: `.tools/notes/completion-2026-10-09.md`, `runtime-games-2026-10-09.md`, `frontend-acceptance-2026-10-09.md`, `audio-runtime-2026-10-09.md`, `cpu-bus-timing-2026-10-09.md`, `emulator-final-audit-2026-10-09.md`. All workers are stopped; no checkpoint commit.

# Active full-platform continuation — October 9, 2026

User resumed the checkpoint and requested the intended full NES development platform. The stop instruction below is historical. Preserve the working tree and targeted results recorded there; no commit yet.

Current ownership: world runtime/assets/language integration; active NMI/gameplay timing and collision/pool acceptance; emulator cartridge/CPU acceptance; root handles integrated build checks, browser authoring/save/compile/run verification, documentation, and final source-only commit. Each worker updates ignored `.tools/notes/` with per-file changes.

Finish the previously incomplete World pipeline and the existing batch's final verification before release. Keep the same core in native/WASM and validate actual guest execution. Actual host CI, audio output and physical-controller evidence must not be inferred from mocks.

# Credit-low checkpoint — October 9, 2026 (stop requested)

**STOP: user requested a checkpoint and pause. No commit was made.** Preserve the current working tree based on `bd42af4`; it contains substantial implemented repairs plus unfinished world integration. Resume only after the user asks. All generated outputs, downloaded fixtures, logs, and `.tools` notes remain ignored.

## Verified work completed before the stop

- Frontend source matrix: **36/36 tests passed**. Save/compile races, stale project/file loading, mixed keyboard/gamepad holds, dual Shift, audio cleanup, exact source-map/file breakpoints, RAM/PPU bounds and viewport, PNG conversion, graphics references/preview, and SFX arpeggio editing/preview have regressions.
- Audio: **10 production CPU/APU execution tests passed**, plus encoding/DPCM/priority/SFX/Sound.Stop suites. `Sound.Play` starts music; `PLAY_SFX` now dispatches effects. Stop silences hardware; effects activate, observe priority, and resume music. Envelope timing/arpeggios/DPCM selection and input validation were repaired.
- Emulator battery tests: **3 passed**. Only declared PRG/CHR NVRAM is exported/restored/preserved on reset; invalid payloads/replacements leave state intact. Unsupported NES 2.0 backend allocations are rejected transactionally.
- CPU contract tests: **4 passed**, including the independent 256/151/56 inventory, 524,288 ADC/SBC operand/flag cases, page/zero-page timing, and all 12 JAM/reset paths.
- Supplemental bus timing tests: **4 passed**. Actual device-visible official/stable-unofficial RMW dummy writes, DMC fetch/stalls, DMC/APU IRQ acknowledgement/masking, and PPU NMI priority over held APU IRQ are covered without injected IRQ entry. Interrupt hijacking and reset isolation remain gaps.
- Root compiler tests: **3 runtime-linking tests plus 3 control-flow tests passed**. Reachability pruning frees fixed-bank capacity while preserving transitive and inline-ASM helper references. Dynamic stores use destination width; INT array reads sign-extend; decimal strings convert zero, unsigned boundaries, and negatives correctly.
- Game runtime: final worker results are **14/14 game execution tests, 12/12 bank/interrupt tests, and NMI safety passing**. Scroll calls/pointers, repeated seams, two-stage attribute uploads, text clipping, metasprite/animation pointer boundaries, and collision overflow were repaired. Exact 28-address unrolled NMI/IRQ saves pass; idle NMI dropped from 1774 to **1136 cycles**. Active-audio/upload timing remains unmeasured.
- Release WASM compilation completed successfully (session `67286`, exit 0). **Bindings were not regenerated and the expanded WASM Node tests have not run.**

## Exact next steps

1. Read ignored `.tools/notes/completion-2026-10-09.md` and worker notes (`runtime-games`, `audio-runtime`, `cpu-bus-timing`, `frontend-acceptance`, and world notes if added). Run `git status --short`; retain all modified/untracked source/tests. No staged release or commit exists.
2. Finish phase-29 world integration. New `src/compiler/world.rs` has draft `runtime_assembly()` and an unused-bank, linker-managed directory/map packer. `compiler/mod.rs` exports it and the API calls `append_assets`. **Runtime emission, World namespace analysis/call dispatch, PPUMASK shadow `$F9`, and selected-map Collision.Tile wiring have not landed.** The worker's combined follow-up patch failed validation, leaving none of those edits; no World Cargo check/tests ran. Add/run four-map transitions, attributes, empty-cell, backtracking, bank/control restoration, and capacity tests. This area is unfinished.
3. Finish the current game scratch/collision timing batch. Confirm the exact 28-address save/restore set, NMI frame counter `$FD`, two-stage 46-byte scroll buffer layout, active audio/upload vblank budget and last-PPU-write timing. Preserve working sound/channel RAM.
4. Run formatting, targeted pending tests, strict all-target Clippy, then the full workspace suite. The earlier full run stopped at a stale `CMP $00` assertion in `control_flow_test`; that assertion was corrected and targeted tests pass, but the full matrix was **not rerun**. Log `.tools/completion-native-20261009.log` is the earlier failed run, not final evidence.
5. Generate browser bindings with `.tools/bin/wasm-bindgen.exe` 0.2.106 from the successful release WASM, then run all Node tests. `CpuState.cycles` now exports **BigInt**; `tests/js/wasm.test.cjs` was updated and adds ALU/JAM/battery checks. Rebuild again if emulator source changes.
6. Run clean-setup/portable verification and actual browser smoke tests. Computer-use skill was read, but no browser session/server was opened. `scripts/verify.mjs` provides frontend/native/full modes; source frontend mode passed before the final two tests were added (worker's complete run is 36/36). Do not re-prepare the core while Cargo is building it.
7. Update README/DESIGN/Brain/architecture/schema/acceptance evidence with actual final results; select source/build/tests/docs explicitly and commit only after the complete batch passes. Actual remote host CI, audible-device and physical-controller evidence remain unverified. Full phases 1–40 acceptance is not claimed.

# Active completion work — October 9, 2026

User resumed and requested full completion, then asked to document the audit findings. Baseline is clean `main` at `bd42af4`; older pause/no-commit notes below are historical. Current delivery remains verified source/build/test/documentation commits, excluding generated outputs.

Read the first section of `IMPLEMENTATION_PROGRESS.md` for the defect inventory and reproduction plan. Three workers own game runtime, sound runtime, and frontend workflows; root owns CPU/cartridge coverage and consolidation. Exact per-file recovery notes are in ignored `.tools/notes/`.

Priority runtime gaps: broken scroll helper argument/return-stack handling, hardware silence and idle SFX activation, music state restoration after SFX, DPCM sample selection. Frontend gaps: save failure aborting compile, one input-listener lifecycle, audio cancellation/rebasing, simultaneous Shift handling, graphics attribute/reference validation.

All listed fixes are pending at this checkpoint. Preserve production-backend tests and record actual outcomes; do not mark phase 40 or earlier phases accepted based solely on passing assembly-string checks. Continue implementation and update this section with final evidence before committing.

# Upstream merge — September 16, 2026

Fast-forwarded main from 4610516 to 6134fa4. Reconciled the pre-merge working tree: upstream already contains the prior implementation, including trainer reset, and adds mapper/bus, math/bitwise, lexer and frontend test work. Retained upstream additions when resolving overlaps. Recovery stash bf34b04798b00a6ecefe1b5dc7cef22244f17e33 (pre-upstream-merge-2026-09-16) is retained; original external fixtures are backed up in .tools/merge-backup-20260916.

Source-only policy restored: generated static/wasm files, emulator/wasm-pack and its stamp, and downloaded nestest payloads are removed from the index but retained locally. Phase acceptance remains incomplete. The merged `Memory.Fill` implementation was corrected to preserve its length across indexed address generation; the regression test and post-fix checks are recorded in the latest progress checkpoint.

# Latest checkpoint — trainer reset fix, September 15, 2026

User requested a concrete short implementation before stopping at low credits (last reported 6%). No commit. Source/build/test-only final commit policy remains unchanged.

- Fixed trainer boot data being lost on Emulator::reset (hard reset). Before the fix, the regression observed random RAM at $7000 instead of trainer byte $5A.
- Emulator now retains the 512-byte trainer only after successful transactional ROM load, replaces/clears it on subsequent successful loads, and reapplies it through the actual CPU bus after hard reset. Failed replacement leaves the loaded cartridge and its trainer intact.
- Regression checks all 512 bytes after reset, including a failed replacement first, then executes the ROM and verifies its read of restored trainer data.
- Entire native swiss-emulator suite, strict workspace/all-target Clippy and formatting pass. Rebuilt WASM and regenerated ignored bindings; all five Node/frontend/WASM tests pass, including the complete canonical trace. Full compiler/workspace suite last passed immediately before this isolated emulator fix and was not rerun this turn. All jobs completed.
- Core preparation's real-path containment guard was also exercised successfully in the preceding quick check. Run node scripts/prepare-core.mjs before Cargo on fresh checkouts; see previous checkpoint and README.

Remaining phase acceptance is unchanged except trainer reset policy now implemented: reset is a fresh boot and restores the original trainer image. Separate RAM/NVRAM persistence, other mapper RAM defaults, full browser integration, active NMI workloads and other phase gates remain pending. Stop here; resume only when user asks.
# Latest checkpoint — low-credit pause, September 15, 2026 (patched core)

**User requested another low-credit pause. No commit. Final commit must contain source/build/test requirements only. Generated artifacts, downloaded fixtures, generated .tools dependency sources and local checkpoint documents must stay excluded.**

Resume prerequisite: `node scripts/prepare-core.mjs` reconstructs ignored `.tools/tetanes-core` from checksum-pinned upstream crate plus source patch. It must run before Cargo on a fresh checkout and after patch edits, with no active build. README and both CI jobs were updated. See the next section for exact patch, checksum and semantics.

Latest verified results:
- Entire native swiss-emulator suite passed against patched core: cartridge, CPU trace/status, interrupt, debugger and palette tests.
- Strict workspace/all-target Clippy and formatting passed.
- Rebuilt release WASM and regenerated ignored browser bindings. All five Node tests pass, including all 8,991 canonical CPU records and new actual WASM NROM capacities (none, 128 bytes, 8 KiB NVRAM).
- Source-only Git check found no tracked WASM/EXE/DLL/NES/ZIP files. The five original generated WASM files remain staged for removal; no other final staging/commit has occurred.
- Full native workspace/all-target locked/offline suite PASSED, exit 0. Log: docs/latest-native-test.log (ignored). Exec session 72562 completed. No build/test jobs remain running.
- Added a final path-containment check to prepare-core.mjs: build storage must resolve to a real .tools directory inside the project. Full preparation also PASSED with the path-containment guard enabled in the final quick check. Do not rerun preparation during a build. Network fallback remains untested because the archive was available in Cargo cache.

Remaining limitations: combined backend RAM/NVRAM capacity has no separate persistence/reset policy yet; other mapper defaults are still unresolved. NROM no-CHR-ROM needs explicit 8 KiB CHR RAM; >8 KiB PRG RAM rejected. Uniform trainers with absent RAM rejected before open-bus readback. Phase 39/40 and earlier integration gates remain incomplete. Continue from these specific gaps and the preceding handoff checklist; do not mark completion or commit just because focused tests pass.
# Patched core / NROM RAM semantics — September 15, 2026

User resumed implementation. No commit. Final commit remains source/build/test requirements only, with generated dependency sources, binaries, downloaded fixtures and local checkpoints excluded.

## New prerequisite

Run `node scripts/prepare-core.mjs` before Cargo on a fresh checkout (and after changing the patch, while builds are stopped). Root Cargo.toml patches tetanes-core to ignored `.tools/tetanes-core`, excluded from workspace membership. Cargo.lock now resolves that local source package.

The script verifies the official tetanes-core 0.12.2 crate SHA-256 `5ab83febf2a67da4ec29d509e9848c18109a7a564f0472a98cdc5cea7c701f47`, taking it from Cargo's archive cache or downloading the pinned archive. It extracts into ignored staging, checks/applies `patches/tetanes-core-0.12.2.patch`, and only then replaces the generated dependency folder. No upstream binaries are committed. Git and tar are prerequisites. `.gitattributes` forces LF for patch files (a CRLF last context line initially broke patch application; normalized LF fixed it). Preparation from cached archive was tested twice successfully; network fallback has not been exercised here. Failed preparation staging directories may remain under ignored .tools/core-prepare-*.

## Changes

- Backend Cart RAM size calculation now decodes the independent low/high nibbles and combines volatile/nonvolatile capacity. Previously the entire byte was treated as one shift, rejecting e.g. nonvolatile-only 0x70.
- Backend NROM honors explicit NES 2.0 PRG RAM absence and capacity. It returns the bus for absent RAM and mirrors smaller declared RAM within the unbanked window. Legacy NROM retains its old 8 KiB default.
- Emulator exposes actual prg_ram_len for diagnostics/tests. NROM loads reject >8 KiB PRG RAM and CHR-ROM-less headers without an explicit 8 KiB CHR RAM/NVRAM declaration. A trainer with explicitly absent PRG RAM is rejected before open-bus readback could falsely appear to verify uniform bytes.
- Executed tests cover no-RAM open bus, 128-byte and 2 KiB mirrors, 8 KiB NVRAM and mixed volatile/nonvolatile combined capacity. These do not prove separate persistence semantics: bus storage is still combined and hard-reset/battery policies remain open. Other mappers' defaults and NES 2.0 edge cases remain unfinished.
- README and native/WASM CI now prepare the patched dependency. Source patch documentation is in patches/README.md.

## Verification

- cargo check and strict workspace/all-target Clippy pass.
- Entire swiss-emulator native suite passes: 7 cartridge tests, 6 interrupt tests, 4 CPU trace/status tests, debugger and palette checks. Canonical nestest still matches all 8991 records.
- WASM rebuild and Node verification running in exec session 14323 when this checkpoint was written. Record final result below. Full compiler/workspace suite against the patched core still needs a run.

Remaining phase gates from prior checkpoints still apply. No phase 39/40 completion claim.
# Latest checkpoint — low-credit pause, September 15, 2026

**User requested a pause and documentation before stopping. No commit. Resume from this section. Final commit must contain only source and build/test requirements; generated binaries, downloaded fixtures, logs and local checkpoint documents stay excluded. Previously tracked static/wasm outputs are staged for removal, with local copies retained and ignored.**

## Latest implementation

- Interrupt wrappers now preserve temporary ranges $00-$0F, $14-$17 and $F0-$F7, plus cross-bank return scratch $07F1 and A/X/Y. A shared 28-byte ROM address table drives sparse save/restore loops. Persistent controller history ($10-$13), text offset ($18), sprite state ($19-$1C), scroll ($E0/$E1), RNG ($E2/$E3) and PPU shadow ($F8) remain shared deliberately.
- Reproduced helper-argument corruption before the fix: NMI changed $14 from 80 to 17. The new execution regression now passes for both NMI and IRQ across all 12 newly protected bytes and verifies that a handler's persistent text-offset change remains visible.
- The first large fixture hit the existing fixed-bank C000-D000 code limit. Separate range loops were replaced with a sparse table; the test uses a compact inline-ASM clobber block rather than repeated POKE calls to fit. Fixed-bank code headroom remains tight; do not claim arbitrary larger handlers now fit. A demand-based runtime/helper layout is still worth addressing.
- Timing regression exposed idle Sound_Update running envelope work for inactive channels: idle NMI took 4,289 cycles, beyond NTSC vblank. Sound_Update now iterates the four channel blocks in one loop and skips inactive channels before note work and after a channel stops. This also saves ROM space.
- Idle NMI now measures 1,774 CPU cycles including interrupt entry, scratch handling and OAM DMA. Nominal NTSC vblank is roughly 2,273 cycles, leaving only about 499 cycles for additional work. Active envelopes, buffered VRAM writes and user handlers require separate budgeting; this is not full vblank acceptance.
- New channel execution regression checks alternating and all-active channel masks: every active timer decrements exactly once; all bytes in inactive channel state remain untouched. Zero-initialized inactive channels must not be interpreted as envelope 0.

## Verification

- All 12 cross-bank/interrupt integration tests pass, including prior atomic ON binding and every-instruction MMC1 interruption tests.
- Updated nmi_safety_test passes for the sparse 28-byte scratch loop.
- Strict workspace/all-target Clippy and cargo fmt pass.
- Full locked/offline workspace all-target suite PASSED (exit 0). Output is docs/latest-native-test.log (ignored). Exec session 70106 completed; no build/test jobs remain running.
- Compiler-only changes this turn: generated emulator WASM did not need rebuilding.

## Resume priorities

1. Confirm final full-suite result below or from docs/latest-native-test.log; repair any regression before new work.
2. Budget active audio/VRAM/user-handler NMI workloads and address limited fixed-bank code headroom. Scratch preservation does not make all shared runtime state reentrant: static SUB argument storage and multi-byte persistent state still need explicit ownership/constraints.
3. Continue phase 39 cartridge RAM/NVRAM and bus/mapper contracts; supplemental phase 40 reset/DMC/IRQ edge cases; source-map/download/lifecycle browser acceptance; and remaining earlier phase gates in the handoff/checklist.
4. Keep final commit source-only, explicitly selecting files. No commit until implementation and required checks are complete. All phases are not yet accepted.

Verification result: full locked/offline workspace all-target native suite PASSED after dynamic interrupt binding. Strict Clippy, formatting and git diff whitespace checks pass. Prospective tracked tree still contains no WASM, EXE, DLL, NES or ZIP artifacts; five pre-existing generated WASM files remain staged for removal only. No commit.
# Dynamic interrupt binding — September 15, 2026

Final commit remains source/build requirements only; generated outputs and local checkpoints stay excluded. No commit yet.

- Implemented previously ignored ON NMI/IRQ DO statements. Semantic analysis rejects unknown events and handlers that are absent, not routines, or require arguments. Codegen checks handlers again for direct callers.
- Bound targets have immutable low/high address tables in fixed-bank ROM. SUB targets dispatch through existing bank-switch trampolines; INTERRUPT targets use their fixed-bank labels. Only referenced targets enter the table, including ON inside nested control-flow blocks. Maximum 255 distinct bound targets.
- ON publishes a one-byte selector at 07F8 (NMI) or 07F9 (IRQ). Zero retains the startup/default pointer at 07F4/07F6. Dispatch snapshots the selector in X, resolves its address through ROM into saved scratch 00/01 and jumps. Single-byte publication also allows handlers to rebind safely without a half-updated pointer.
- Native execution tests verify both NMI and IRQ calling bank-3 routines, rebinding from inside the handler to bank 5, completing Main and restoring bank 0. Invalid handler diagnostics pass. A deterministic interrupt-injection test covers before LDA, before STA and after STA of publication, using linker source-map location; it observes exactly the old or new handler.
- All nine cross-bank tests and strict workspace/all-target Clippy pass. Full locked/offline all-target suite is running; record final result below. Compiler-only changes do not require a new WASM emulator build.

Remaining: wider helper scratch ownership and interrupt/vblank cycle budgets, cartridge/bus contracts, supplemental CPU edge cases, frontend integration acceptance and other phase gates. No complete-phase claim.

Latest verification (September 15): full locked/offline workspace all-target suite PASSED with the MMC1 retry fix. All NMI/IRQ setter instruction-boundary injections pass. Dynamic ON NMI/IRQ DO binding was found parsed but ignored by both semantic analysis and codegen; implementing it needs validated zero-argument handler targets, fixed-bank trampolines, and interrupt-safe publication of the two-byte handler address. This is a confirmed next task, not implemented yet.
# MMC1 interrupt recovery — September 15, 2026

Final commit remains source/build requirements only, after implementation and checks are complete. No commit yet; generated files remain ignored.

- Reproduced mapper corruption by injecting NMI immediately after the first serial PRG write while its handler called a subroutine in another bank. Main failed to finish.
- MMC1_SetPrgBank now saves the desired bank in $07F0, clears retry flag $07F2, resets the MMC1 serial register, writes all five PRG bits, and retries if an interrupt set the flag. Both interrupt epilogues set $07F2 before restoring registers. Handler bank calls restore the interrupted bank shadow; the interrupted setter repairs any partial serial sequence before returning to switchable code. X/Y remain untouched by the setter.
- The regression resolves the setter via generated trampoline machine code and injects each NMI/IRQ at every instruction boundary through the setter's RTS. It uses the pinned production ControlDeck CPU/bus/mapper, invoking its interrupt entry rather than substituting a CPU. All boundaries and all six bank execution tests pass. Existing PPU/APU interrupt tests separately exercise real arrival/polling.
- This relies on compiler-generated mapper writes and the fixed-upper-bank MMC1 mode. Arbitrary inline-ASM register writes are not synchronized. Additional interrupts can cause retries; vblank cycle budgeting and wider runtime scratch ownership remain open.
- Full locked/offline workspace all-target suite is running; record its final result below. No emulator source changed this turn, so the local generated WASM does not require rebuilding for this compiler-only fix.

Verification update: the full locked/offline workspace all-target suite passed after updating the old unrolled-instruction NMI test for indexed loop bounds. Both real NMI and APU IRQ scratch-preservation regressions pass (five cross-bank tests total). An earlier concurrent build caused a Windows executable lock error; the final successful run was serialized. Runtime partial MMC1 serialization remains open. No commit.
# Interrupt scratch preservation — September 14, 2026

Source-only final commit policy remains in force. No commit yet; generated files stay ignored and their previously tracked versions are staged for removal.

- Reproduced a runtime interrupt corruption: Main held 93 in cross-bank return scratch $07F1, NMI wrote 17, and Main observed 17 after return. New regression failed before the fix and passes after it.
- Both generated NMI and IRQ wrappers now stack/restore $07F1 alongside saved registers and $00-$0F scratch. This isolates cross-bank expression return storage across interrupts.
- Added instructions initially pushed a larger nested-bank test into the fixed D000 data segment. Replaced duplicated unrolled 16-byte scratch saves/restores with indexed loops in both wrappers; all four cross-bank execution tests now pass. Loops add interrupt cycles and need vblank workload budgeting; this is not complete interrupt/runtime acceptance.
- Partially interrupted MMC1 serial writes, wider runtime scratch, and interrupt metadata/banked dynamic handlers remain unresolved. Do not report MMC1 reentrancy fully fixed.
- Full locked/offline workspace all-target suite and strict Clippy are running at this checkpoint. Record their actual final results before proceeding.
# Source-only commit policy and IRQ coverage — September 14, 2026

User explicitly requires the eventual commit to contain only source and materials needed to build; compiled outputs must be excluded. Commit only when implementation and checks are complete. Select paths explicitly: code, tests, build/CI scripts, dependency manifests/lockfile, ignore rules and build instructions. Local progress/audit/checkpoint documents are not final-commit material. No commit yet.

- Removed all five tracked generated static/wasm files from the Git index using git rm --cached; local copies remain. These removals are staged. static/wasm is now ignored. No tracked WASM/EXE/DLL/NES/ZIP files remain in the prospective tree.
- External nestest binary, trace and author readme are ignored. scripts/fetch-test-fixtures.mjs downloads pinned upstream inputs, verifies SHA-256 before writing, verifies existing inputs without a network call, and refuses checksum mismatches. Provenance stays in source documentation. The existing-input verification branch passed; fresh network download has not been separately exercised here.
- README documents pinned Rust/target/wasm-bindgen build and fixture/test steps. Native CI fetches fixtures and runs source frontend tests; WASM CI builds fresh bindings before the bundle trace test. No checked-in generated bundle dependency remains.
- New APU frame IRQ regression verifies I masking, CLI polling delay, cartridge IRQ vector/stack B=0, and immediate held-IRQ reentry after RTI. All six CPU interrupt integration tests pass. Existing five frontend/WASM tests pass with local generated artifacts.

Remaining phase requirements persist. Next: reset/DMC/interrupt priority edge cases, compiler interrupt/MMC1 reentrancy, bus and cartridge RAM contracts, browser integration. Do not commit on this checkpoint alone.
# CPU integration and regional pacing — September 14, 2026

Latest continuation: commit only when implementation and required checks are complete; no final diff tree. No commit yet.

- Five new production-ROM CPU regressions pass: exhaustive BRK status stacking and RTI (256 status values), page-one stack wrapping, indirect-JMP page wrap, PPU-triggered NMI/vector/stack/I masking/edge behavior, and OAM DMA 513/514-cycle alignment stalls. IRQ priority/polling, DMC, interrupt hijacking, reset distinctions and other phase gates remain incomplete.
- Added frame_rate() from active backend region and clock. Frontend pacing consumes it instead of NTSC constant. Native NTSC/PAL/Dendy ROM tests and 60/120/144Hz frontend timing tests pass. NTSC rate is nominal rendered-frame cadence (alternate one-dot skip); no-render frames have a tiny different cadence.
- Rebuilt WASM and regenerated bindings. New Node regression executes the exact browser bundle against all 8,991 canonical records: all PC/A/X/Y/P/SP/cycles match. It also checks palette length and frame rate. This validates WASM execution, not browser UI/audio integration.
- All five Node tests pass; strict workspace/all-target Clippy and formatting pass; targeted cartridge and CPU interrupt tests pass. CI now runs Node tests after freshly rebuilding WASM as well as testing the checked-in bundle in native jobs. Remote CI has not run.

Next: browser download/source-map/lifecycle proof, IRQ/DMC/reset supplemental coverage, mapper/bus architecture and RAM semantics, compiler interrupt/MMC1 reentrancy. Full phase acceptance remains pending.
# Delivery instruction updated — September 14, 2026

User explicitly changed final delivery: commit when implementation and required checks are complete. This supersedes all earlier requests for a final diff tree. Do not commit incomplete implementation. No commit has been made yet.

Latest CPU validation: four cpu_trace tests pass, including the complete 8,991-record trace, exhaustive PLP/PHP/RTI stack status checks, and comparator negative cases for every field and all eight status bits. Strict workspace/all-target Clippy passes. Phase 40 remains incomplete pending supplemental coverage and integration evidence.
# Latest continuation — September 14, 2026

This section supersedes the older pause notes. User resumed implementation. No commits; final delivery remains a diff tree, not a commit. Phases 1–40 are not fully accepted.

- Resolved nestest record 104 mismatch as a debugger representation issue: B/U are not physical CPU flags. `get_cpu_state` now publishes U=1/B=0 while preserving all six real flags. Backend PLP retains pulled B/U; RTI clears both; PHP and interrupt pushes explicitly serialize them. Reference: https://www.nesdev.org/wiki/Status_flag . No backend execution patch or comparator masking was needed.
- All 8,991 canonical records match PC/A/X/Y/P/SP/cycles. Result RAM bytes are checked unchanged from initial random RAM: successful automation never writes/initializes $02/$03. The earlier zero assertion was invalid for production random RAM.
- Exhaustive 256-value PLP/PHP and RTI stack-status tests pass; RTI checks PC, SP and 28-cycle total. Supplemental IRQ/NMI/BRK/DMA and comparator negative tests remain.
- Palette export now exactly 128 bytes, laid out 16x2 RGBA, with 32-byte stack scratch. Frontend uses the explicit layout, validates length and renders pixelated swatches. Rust and Node regressions pass. Node regressions added to the host CI matrix.
- Validation: full locked/offline workspace all-target suite passed before palette edits; all emulator tests passed after palette edits; strict workspace/all-target Clippy and formatting passed; all 3 Node tests passed. WASM release rebuilt and bindings regenerated using CLI 0.2.106. Browser visual verification of the latest bundle remains.

Next: finish CPU trace comparator negative tests and supplemental phase-40 coverage, source-map/download/lifecycle browser proof, interrupt/MMC1 reentrancy, and the incomplete cartridge/bus contracts described below. Existing phase checklist stays pending where broader evidence is missing.
# Latest checkpoint — third low-credit pause, September 14, 2026

**This section supersedes the older checkpoints below. User requested a pause. No commits. Final deliverable remains a final diff tree, including patch/new files/binaries and handoff instructions, after implementation is complete. No final package yet.**

## Immediate next action: failing CPU trace

New `emulator/tests/cpu_trace.rs` compares all PC/A/X/Y/P/SP/cycles against the 8991-record canonical nestest trace. It currently FAILS at record 104, PC C826 (BNE), cycles 270: expected status EF, actual FF. All other fields match. Difference is status bit 4 (B). Investigate backend PHP/PLP/RTI status handling and CpuState snapshot semantics. Do not silently mask fields or weaken the comparison without establishing the architecture's actual representation and adding targeted tests.

Fixture source: `D:\AI github\nes-test-roms-reference`, cloned from christopherpow/nes-test-roms at revision 95d8f621ae55cee0d09b91519a8989ae0e64753b. Checked-in subset is `emulator/tests/fixtures/{nestest.nes,nestest.trace,nestest-readme.txt,PROVENANCE.md}`. Full SHA256 and initial/last record states are in PROVENANCE.md. Test modifies only the in-memory reset vector to C000 per documented automation entry. New Emulator::trace_instruction returns pre-instruction CpuState and clocks the production backend once. It is also exported to WASM, but WASM has not been rebuilt since this addition.

## Changes during latest continuation

- Source-map include/macro call-site regression added and passes. Full locked workspace/all-target suite passed before subsequent cartridge/CPU/asset changes.
- app.js now downloads sourcemap.json alongside game.nes. Superseded compile responses are discarded using a revision counter and project check. Downloads have not been tested end-to-end in the browser yet.
- project.js sets currentFile before firing input. Editor clears maps/breakpoints on project-loaded, checks compiled project identity and source contents before mapping.
- Browser verification: simple loop with breakpoint on assignment line 4 stopped and showed `line-num breakpoint debug-active`; Play button reflected paused state. Continue re-hit the same line. Browser errors none observed in that path. Hover, multi-file UI, stale-edit and download tests remain.
- Editor now uses elapsed-time pacing at current NTSC 60.0988 Hz, at most four catch-up frames, and one tracked RAF callback. Node tests `tests/js/editor.test.cjs` pass for 60/120/144 Hz, duplicate scheduling and long stalls. Add them to CI. PAL/Dendy timing still needs correct backend-derived rates.
- Blur, hidden-tab, Close and gamepad disconnect input release added. Hidden tab pauses. Gamepad polling clears disconnected state. Actual hardware input tests remain.
- Frontend ROM loading reuses transactional Rust loader; only after success cancels old RAF and swaps source map. No longer frees running instance before attempted load. Audio backlog/error lifecycle still needs review.
- Added `emulator/src/cartridge.rs` normalized checked iNES/NES2 metadata parser before backend allocation. Max64MiB per region; header, payload, exponent/linear sizes, separate RAM/NVRAM fields, trainer and metadata decoded. Invalid console/misc-ROM rejected. Whole-bank exponent sizes converted for backend; trainer stripped from backend bytes and written/read-verified via actual bus at7000..71FF before transactional swap. Four cartridge tests pass, including actual execution reading trainer data. Partial-bank geometry is rejected, not supported. RAM absence/NVRAM semantics still need backend reconciliation. See docs/EMULATOR_ARCHITECTURE.md.
- Compiler generate_banks resets string/table/signature/label state on reuse. Deterministic reuse test passes.
- ProjectAssets::validate_for_compile added: CHR bound, palette colors, NT960bytes with64attrs (or empty legacy attrs), metatile palette, metasprite/frame bounds, nonzero animation duration, world shape/references. Asset validation tests pass. Broader tests may expose legitimate legacy inputs needing documented migration; do not silently relax diagnostics.

## Verification state

Passed this continuation:
- Full workspace all-target tests after source-map migration (before later cartridge/CPU additions).
- source_map_contract_test: 2 pass.
- tests/js/editor.test.cjs: 2 pass. node --check app/project/editor pass at edits.
- emulator cartridge: 4 pass; debugger: 1 pass.
- asset_validation_test: 2 pass.
- cargo fmt --all last run after adding CPU trace test.

Known failing: canonical CPU trace described above. Current full suite is therefore NOT green. Strict all-target workspace Clippy passed at pause. Latest generated WASM is stale relative to cartridge parser/trace API; rebuild and regenerate with installed .tools/bin/wasm-bindgen 0.2.106.

## Process/continuation notes

Local cargo server session25604 was stopped before recompiling compiler tests. Temporary resumedTab browser was paused and is closed at checkpoint. No server should remain. Avoid rebuilding the main exe while cargo run holds it on Windows.

New source modules/tests/docs and fixtures are untracked; final diff packaging must include them, not just `git diff`. Current baseline HEAD remains4610516840d7991bce7e1d879e1f4e2fa831f10e. Do not stage/commit as a delivery shortcut.

Read prior checkpoint priorities below for remaining interrupt/MMC1, tables, assets, gameplay/audio, backend mapper/bus contract, CPU coverage, browser and host-matrix work. All phase acceptance remains incomplete until evidenced. `docs/IMPLEMENTATION_PROGRESS.md` records the chronology.

---
# Latest resume checkpoint — paused September 14, 2026

**Read this section before the historical September 13 checkpoint below.** User requested another pause for low credits. No commits or pushes have been made. Phase-through-40 work remains incomplete.

## Delivery instruction superseded

The user now wants a **final diff tree instead of a final commit**, suitable for handing off to another developer. Never commit this work. On completion produce a binary-capable Git patch including untracked new files, a changed-file tree, baseline HEAD, file checksums, validation evidence, and step-by-step application instructions. No final diff package exists yet; do not call current partial work complete.

## Changes completed in the September 14 session

- Collision Rect/Point/Tile return values survive argument-stack cleanup by using Y while A is used to restore SP. Added negative collision cases. Controller fixture uses supported DO/LOOP WHILE and IsHeld(Button.A), with press/release verification. The former two failing phase11–20 fixtures now pass.
- Legacy CodeGenerator::generate delegates to generate_banks and flattens only banks 0/7; banked programs use generate_banks. Full native workspace suite passed after this unification, before later source-map changes.
- Semantic struct sizing and codegen array/RAM sizing now use checked arithmetic. Exact 560-byte variable RAM succeeds; overflow and large dimension cases fail cleanly. Tests pass.
- CI configured for Windows/Linux/macOS with Rust 1.98.0 and locked workspace tests/clippy/fmt; separate WASM build/binding generation job. Remote CI not run.
- Installed Rust WASM target (approved escalation), built release WASM successfully, installed wasm-bindgen-cli 0.2.106 (matching Cargo.lock) into ignored `.tools`, regenerated tracked static/wasm artifacts. Never copy Jules generated WASM.
- Emulator breakpoint lookup now uses production mapper map_peek PRG-ROM offset, in physical 16 KiB units. It stops before the initial instruction; continue skips only the last reported location once and re-arms it. Added side-effect-free peek_cpu and prg_offset methods. ROM load uses a replacement ControlDeck and swaps only on success, preserving configured sample rate. Invalid sample rates ignored.
- New emulator/tests/debugger.rs validates initial stop, continue/re-hit, NROM mirrors, invalid-load preservation, and reset/re-arm. It passes.
- Browser smoke test with regenerated WASM: starter Run opened controls; no error/warning logs; Pause became Play. This occurred before source-map schema changes. No full browser/physical-gamepad acceptance.

## Source-map work just implemented (still needs completion)

- Statement and TopLevel now carry source_file; parser defaults main.swiss, AST convenience constructors default empty string. Includes recursively stamp filename; macro expansion stamps primary call-site file/line. There is no macro definition secondary location.
- Codegen emits `;@source` JSON comments at top-level and statement boundaries and resets source provenance before runtime helpers. Assembler carries provenance through final branch-relaxed layout and marks executable instructions.
- New src/compiler/source_map.rs defines LinkedSourceMap `{version:1, sources:{filename:exactSourceText}, entries:[{file,line,bank,cpu_start,cpu_end,rom_offset,kind}]}`. CPU ranges are half-open; rom_offset is FILE-relative, including the 16-byte header. Exact source snapshots identify the compilation revision.
- Production compile API now returns this linked map instead of estimated pairs. Codegen's compatibility return still contains the old estimates but production ignores them.
- editor.js now has mappedEntries() filtering by current file and exact source contents. Hover, syncBreakpoints, and updateDebugInfo use the same records and actual mapper bank. Editing clears stale mapped breakpoints/highlights.
- `node --check static/js/editor.js` passes. Browser integration of this new schema has NOT been tested yet.
- New tests/source_map_contract_test.rs checks a bank-3 LET against its actual LDA-immediate ROM bytes and verifies a bank-0 call mapping. It passes.

## Final verification at pause

Passing commands after source-map changes:
- cargo fmt --all
- node --check static/js/editor.js
- cargo test --test source_map_contract_test --test compile_api_test --test phase_1_10_execution_test --test linker_test --offline (20 tests pass)
- cargo clippy --workspace --all-targets --offline -- -D warnings passed after the focused tests.

Earlier targeted phase11–20, allocation, cross-bank, and emulator debugger tests pass, but full workspace tests have not been rerun after AST/source-map changes. Do that next. The earlier initial test attempt failed with Windows exe lock because cargo run was active; server was stopped and the retry above passed. This was not a compiler behavior failure.

## Processes and tools at pause

- Local cargo run server session 98588 was stopped with Ctrl-C. No server should remain running.
- In-app browser testTab was paused and will be closed at checkpoint. Browser work used cua_repl only. No native app automation was used.
- `.tools/bin/wasm-bindgen.exe` is installed. Build commands:
  cargo build -p swiss-emulator --release --target wasm32-unknown-unknown --locked --offline
  .\\.tools\\bin\\wasm-bindgen.exe --target web --out-dir static/wasm --out-name swiss_emulator target/wasm32-unknown-unknown/release/swiss_emulator.wasm
- Rust 1.98.0; native MSVC and wasm32 targets installed. Node is available on PATH.

## Resume priorities

1. Finish source-map integration: include/macro provenance tests; branch relaxation mappings; ensure compiler-generated startup/epilogues have deliberate attribution. Export sourcemap.json alongside downloaded ROM (app.js still downloads only ROM). Browser verify hover/breakpoints/active line, editing invalidation, current-file changes, and stale compile response. Project switch with identical source may need explicit project identity invalidation.
2. Existing codegen estimates remain for legacy callers; don't mistake them for linked locations. Reset generator state on reuse and test determinism. Consider replacing fragile marker parsing defaults with typed deserialization. Inline user ASM could contain source markers; treat this as provenance only, never privileged behavior.
3. Finish original runtime/interrupt work: MMC1 serial NMI interruptions, shared $07F1 scratch and wider NMI scratch preservation; ON NMI and interrupt semantic bank metadata; table/vector overflow; mirroring choice; input edge tests; pool full/reuse and RNG variation; asset validation. User runtime limits and all phase acceptance remain pending.
4. Complete remaining handoff architecture and phase39/40 CPU requirements, plus UI/audio/gameplay acceptance. No new cartridge header adapter or canonical CPU trace suite has been added yet. Backend remains tetanes-core 0.12.2.
5. Fix browser lifecycle/pacing, safe buffer lengths, CPU/RAM/PPU views; latest palette buffer still overallocated. Browser load wrapper currently frees old emulator before loading new, despite transactional Rust load, so frontend failure behavior still needs repair.
6. Update README/DESIGN/AGENTS and acceptance evidence, run native/WASM/browser gates, then produce final diff tree. **No commit.** Hardware and remote host acceptance cannot be silently marked complete.

The September 13 state below is historical and is superseded wherever it conflicts with this section. See IMPLEMENTATION_PROGRESS.md for the running timeline.

---
# Resume checkpoint — implementation paused September 13, 2026

## User direction and repository state

The user requested implementation of `docs/DEVELOPER_HANDOFF_THROUGH_PHASE_40.md`, with **no commit until everything is complete**. They authorized reviewing and incorporating relevant changes from the Jules ZIP. They then asked to pause for low credits and document the state. Resume implementation when requested; do not treat this checkpoint as completion or permission to commit partial work.

- Repository: `D:\AI github\swissarmyNES`
- Base HEAD remains `4610516840d7991bce7e1d879e1f4e2fa831f10e`.
- All planning documents and implementation changes are uncommitted. Nothing was pushed.
- No implementation phase is signed off. The acceptance worksheet remains pending.
- No server or test process is intentionally left running.
- Read AGENTS.md and the handoff before resuming. Phase 39 is cartridge/bus/mapper architecture; phase 40 is CPU core hardening, not the historical MMC1 roadmap.

## Implemented so far

1. Restored production compile API for empty Main, WORD arithmetic, Controller.Read, and Text.Print fixtures. Added `tests/compile_api_test.rs` including actual emulator RAM verification.
2. Replaced assembler emission with a two-pass linker using the existing rs6502 opcode catalog. Upstream assembly silently omitted generated DB/WORD directives. New linker emits data, resolves forward labels, validates banks/CHR/overlaps, and relaxes distant conditional branches in both directions. `assemble_with_layout` returns actual emitted addresses; these are not yet wired into source maps.
3. Put reset/startup in fixed bank 7 and initialized MMC1 before calling user code. Added omitted runtime helpers to generate_banks. Cross-bank nested calls restore the caller bank at $07F0. Preserve A across bank restore using $07F1; X is untouched by the bank routine. Main in a switchable bank uses a trampoline.
4. Fixed STRING assignment to retain its high pointer byte; negative expressions now produce signed type information.
5. Added `Emulator::ram_snapshot()` and an rlib crate output for safe native integration tests. Added exact tetanes-core dev dependency and swiss-emulator dev dependency.
6. Moved interrupt pointers from $03FA-$03FD (inside string heap) to $07F4-$07F7; user RAM stops at $07EF. Handler initializer uses linker-resolved low/high label bytes. Interrupt bodies in generate_banks are emitted in fixed bank 7. Added execution coverage with Main in bank 3 and enabled NMI.
7. Checked codegen array size multiplication and RAM addition; fixed asset pointer table widths to two bytes and added omitted metasprite/animation pointer entries. These require broader regression coverage.

## Jules ZIP review

- Original: `C:\Users\kd7tc\Downloads\jules_session_2423577224430665052.zip`
- Safely extracted outside repository: `D:\AI github\jules-review-2423577224430665052`
- Partial file overlay, not a full repository. Attached documents are reference material, not new user instructions.
- Incorporated/adapted `tests/phase_1_10_execution_test.rs`, `tests/cross_bank_execution_test.rs`, and most recently `tests/phase_11_20_execution_test.rs`.
- Replaced ignored emulator errors and unsafe RAM pointers with strict error handling and owned snapshots.
- Do not wholesale copy its assembler/codegen: it uses guessed addresses/default fallbacks and F000 raw-data injection conflicting with the DPCM region. Useful fixes included string assignment, signed negation, pointer widths, and execution fixtures.
- Jules phase 11–20 fixtures contain invalid syntax/API and weak/wrong assertions; these are deliberately not accepted as proof yet.

## Verification and exact current failures

Passed earlier in this implementation session:
- `cargo test --workspace --offline` after initial compiler/linker/string fixes.
- `cargo clippy --workspace --all-targets --offline -- -D warnings` after initial runtime pointer/table changes.
- `cargo test --test linker_test --offline`: 4 pass (data/vectors, bidirectional long branches, overlap/bounds rejection, expression subtraction associativity).
- `cargo test --test phase_1_10_execution_test --test cross_bank_execution_test --offline`: all passed before subsequent additions.
- Latest `cargo test --test cross_bank_execution_test --offline`: 3 pass, including bank-3 Main and NMI in fixed bank.
- Targeted ram_overflow_test and nmi_safety_test pass. A full run failed only because ram_overflow_test still expected the old $07FF error text; that assertion has since been corrected and passed.

**Latest phase_11_20_execution_test run: 2 pass, 2 fail. Do not describe the current full suite as green.**

1. Controller fixture uses `DO WHILE 1` / `LOOP`, but parser requires `DO` / `LOOP WHILE condition` (or WHILE/WEND). It also uses unsupported `Controller.State(0)`; supported API is IsHeld/IsPressed/IsReleased(Button.*). Rewrite fixture against supported API and verify press/release edges. A is serialized as bit 7 internally. Test currently fails at parsing before runtime.
2. Collision.Tile(16,16) returns 253 instead of zero with absent nametable. **Strong code evidence found just before pause:** immediately after JSR Runtime_Collision_Tile, generated TSX/TXA/ADC #4/TAX/TXS stack cleanup overwrites A with stack-pointer value $FD. Rect and Point cleanup have the same bug; their positive-only tests spuriously pass because $FD is truthy. Preserve returned A through stack cleanup, add negative/boundary collision cases, and compare Tile against an explicitly injected nametable fixture. Do not change expected Tile value to 253 as Jules did.

Formatting has not been rerun since the last few patches. Run cargo fmt before next strict clippy check. Do not claim the earlier full suite/clippy covers later changes.

## Next work, in order

1. Fix/adapt the two pending phase 11–20 regressions above and strengthen pool/RNG fixtures (full pool, reuse, deterministic seed, bounds), early string dedup content assertions, and allocation boundaries.
2. Audit interrupt safety: bank switch serial writes can be interrupted by user NMI bank calls; $07F1 return scratch can be clobbered by interrupt bank calls. Interrupt metadata still uses semantic analyzer's generic Sub declaration; direct/cross-bank calls and ON NMI DO need correct fixed-bank/trampoline routing. NMI currently saves only $00-$0F plus A/X/Y while other helpers use more scratch bytes. No complete reentrancy acceptance yet.
3. Unify legacy generate() and generate_banks(): currently separate paths; production uses generate_banks, many old tests merely inspect legacy assembly. Avoid maintaining two runtimes. Preserve inspection tests or migrate them to the production path.
4. Complete actual source maps (versioned schema, source file provenance, physical bank, final instruction addresses). Existing SourceMap is still line/address pairs; frontend parts expect triples. Linker Location currently only has physical bank/address/length/local assembly line, not source provenance. Instruction records retain source_bank internally. Direct output.push bypasses estimated source tracking.
5. Harden table size and memory checks: pointer table must stay below FFFA; some additions still unchecked. Semantic analyzer get_type_size/struct arithmetic may still overflow even though codegen now checks arrays. Audit asset region overlaps and limits. MMC1 control initialization is $0C (one-screen mirroring), needs consistency with intended scroll behavior.
6. Work through remaining handoff: persistence/asset validation, early gameplay/audio/authoring acceptance, WASM rebuilding/browser lifecycle/input/debugger, then phase39 cartridge/bus adapter and phase40 CPU trace tests. tetanes-core 0.12.2 remains underlying emulator; no separate CPU implementation added.
7. WASM target/browser/hardware/nestest validation has not been performed. Only native MSVC target was installed at baseline; check current tools before downloading dependencies. No generated WASM from the Jules ZIP was copied.
8. Update acceptance evidence and README/DESIGN/AGENTS when complete, run required complete checks, and only then commit. User explicitly requested no partial commits.

## Files to inspect first

- `src/compiler/assembler.rs`: new linker; expression parser handles basic +/- and low/high selectors; not a general assembly expression parser.
- `src/compiler/codegen.rs`: runtime, startup, bank routines, collision cleanup near Runtime_Collision_* call sites.
- `src/server/api.rs`: production path; final assemble_banks call still returns estimated source_map.
- `tests/phase_11_20_execution_test.rs`: newest failing fixtures.
- `tests/linker_test.rs`, `tests/compile_api_test.rs`, `tests/cross_bank_execution_test.rs`, `tests/phase_1_10_execution_test.rs`.
- `docs/DEVELOPER_HANDOFF_THROUGH_PHASE_40.md`, `docs/BASELINE_AUDIT.md`, `docs/PHASE_40_ACCEPTANCE_CHECKLIST.md`.

The older handoff archive `D:\AI github\swissarmyNES-phase40-handoff.zip` contains planning documents only, not these implementation changes. The working repository is the authoritative resume state.

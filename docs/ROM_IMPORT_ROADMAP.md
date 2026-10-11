# Later milestone: 1980s ROM import and project recovery

Requested October 11,2026; planned Phase 48 in [DESIGN](../DESIGN.md). No importer/decompiler is implemented. Current [Studio game systems](GAME_STUDIO_ROADMAP.md) remain the priority.

## Target and boundaries

Import original 1980s NES/Famicom cartridge games into complete editable projects: code, graphics/palettes/maps/animation, game flow, music and SFX. Save/reopen, preview, rebuild and export a playable ROM while preserving the original engine, mapper/bank layout, metadata, trainer, RAM/battery, timing and expansion requirements.

Begin with cartridge .nes and regional variants, staging mapper families0/NROM,1/MMC1,2/UxROM,3/CNROM,4/MMC3,7/AxROM after individual fixtures pass. Era alone does not identify hardware/asset formats. Famicom Disk System requires a separate disk path. [NES2.0 format](https://www.nesdev.org/wiki/NES_2.0) · [submappers](https://www.nesdev.org/wiki/NES_2.0_submappers).

Compiled games usually lack source names/comments/types. Useful recovered representations need provenance and manual annotations; exact original-source identity and automatic recovery of all semantics cannot be promised. Preserve unknown bytes/fields instead of silently replacing engines/codecs with Studio presets. Compatibility means a documented matrix of independent emulators supporting the cartridge, not every existing/future emulator.

## Ordered development and acceptance

| Step | Build | Required evidence |
|---|---|---|
|1. Project format | Versioned recovered mode; ROM hash/hardware/banks/physical offsets/labels/annotations/resources/analysis status; compatible authored projects | Reopen intact; original input/build ROM remain local/ignored; build uses recovered source/assets |
|2. Inspect/extract | Validate lengths/hardware before transactional PRG/CHR/section extraction; assign every byte an owner | Failure preserves existing project/running game; uncertain regions remain tracked |
|3. Code/data recovery | Vector-led bank-aware disassembly plus bounded production-core traces; separate CPU addresses/physical banks | Retain indirect/bank/interrupt paths, RAM code, overlapping interpretations and undocumented opcodes; traces prove observed paths only |
|4. Exact reconstruction | Distinct build mode preserving original layout/mapper/alignment/raw instructions/data/header; current authored compiler stays fixedMMC1 | Import→save→reload→source-only build reproduces every untouched byte without original input; no embedded-ROM shortcut |
|5. Behavioral baseline | Controller scenarios, reset/interrupt/bank events, screen/audio/save observations for games/regions | Original/rebuilt menus/gameplay/transitions/late paths agree; boot alone insufficient |
|6. Graphics/world codecs | RawCHR first, then verified palettes/sprites/animations/maps/collision, CHR-RAM/compression profiles | Unchanged decode/encode round trip plus visual edit; retain unknown fields/storage limits |
|7. Original audio drivers | Verified driver profiles for songs/instruments/sequences/SFX/DPCM and supported expansion audio | Music/SFX edits rebuild in original driver; preserve timing/channel behavior; unknown drivers retain editable code/data |
|8. Studio game systems | Cross-references/functions/variables/scenes/events and optional supported SwissBASIC conversion | Every item links to bytes/evidence; preserve timing-sensitive assembly; reject conflicting shared/overlapping edits; demonstrate gameplay/code edits |
|9. Export/compatibility | Pin independent emulator versions/configurations and hardware support; export source/ROM/test/recovery reports | Unchanged/edited boot/input/render/audio/interrupt/save tests; native+WASM shared core count as one implementation |
|10. Expand support | Per-game/mapper catalog separates exact rebuild, decoded resources, semantic recovery and emulator checks | Banking/compression/hardware/formats expand only through fixtures; partial recovery remains explicit |

Completion requires representative supported 1980s games, exact untouched source-only rebuilds, persistent project/provenance, working code/art/map/music/SFX edits, independent-emulator results and the full in-app import/analyze/edit/preview/build/export workflow. A raw byte dump alone does not satisfy Studio integration. Unsupported capabilities must fail clearly without overwrites. This later milestone does not sign off phase 40 or the unfinished game platform.

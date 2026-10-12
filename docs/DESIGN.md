# SwissArmyNES development guide

This is the single merged guide: current state, plan, Studio authoring/schema, implementation contracts, acceptance and development history. [README](../README.md) owns setup commands; [AGENTS](../AGENTS.md) owns operating/recovery/publishing rules. Patch/example/fixture READMEs stay beside their source inputs. Do not create another roadmap, status file or contract copy.

- [Current state](#current-state)
- [Plan and original phase index](#plan)
- [Studio authoring and schema](#studio-authoring)
- [Runtime contracts](#runtime-contracts)
- [Shared sprite and animation data](#shared-sprite-and-animation-data)
- [Acceptance and release](#acceptance-and-release)
- [Development record](#development-record)

## Current state

October 11, 2026; development builds on published9243658. The user requested a complete NES environment, combat/AI, reuse of previous work, one merged guide under docs, fewer files, compact recovery notes and no old backup accumulation. The user subsequently authorized committing all source/docs to a test branch, pushing/opening a PR, waiting for GitHub checks on its exact head, then merging main and deleting the temporary branch. Consolidation and this development slice are complete: optional facing melee, bounded pursuit and melee cooldowns use the existing shared combat/spawner path. Shared projectile/response pools are next. Publication state is recorded in recovery notes and the PR.

Present: production compiler/linker/banked runtime, integrated art/world/audio editors and shared native/WASM emulator/debugger. Studio provides connected scenes, original editable starters, protected generated source, reusable entities/room slots/spawners, hitboxes/health/immunity/defeat/restart, shared palette/mirror/facing variants/metadata aliases, persistent WORD variables, ordered events and saved controller/state tests. Latest slice adds optional horizontal-facing melee and bounded horizontal pursuit with shared spawned behavior; defaults preserve existing designs and declaration offsets. Facing uses one appended byte shared with animated facing and works without player animation; pursuit adds no enemy RAM. Width1 melee was corrected and tested.

Latest local verification: 294 native, 93 frontend and 3 production-WASM tests passed (390 total), with workspace check, formatting and strict Clippy. The 37 Studio native tests include facing/pursuit/cooldown behavior, an exact expiry boundary, shared placed/spawned targets, room/restart guards and a deliberately wrong saved expectation that must fail. The 38 Studio frontend tests cover persistence/defaults and RAM budgets. Combined capacity includes two six-enemy pursuit pools, cooldown/facing melee, all palettes/audio and both draw paths. A full run exposed an audio-test startup race; the corrected fixtures deliberately preload the colliding RAM marker and wait for the linked store instruction to finish. Channel/NMI assertions are unchanged. New live-browser/physical-device/full-release acceptance is not claimed. Published9243658 passed host/WASM CI38157045058; this branch's GitHub checks are a separate publication gate.

Preserved projects: platform-demo and studio-{acceptance-20261010,controls-20261011,entities-20261011,events-20261011,combat-20261011,spawners-20261011}. No authored/custom source changed by latest implementation/audit. Copied acceptance server is stopped; inspect process state before restarting. Core/tools/cache/generatedWASM remain ignored and retained. All17new document backup copies and the temporary detailed inventory snapshot were removed; no replacement archive. Compact current recovery notes are `.tools/notes/game-studio-2026-10-10.md`.

Current limits: Studio tests use an isolated harness, not general custom-main/device/fidelity acceptance. Room pickups respawn, events are scene-scoped/AND-only, pursuit is horizontal within explicit bounds, authored compiler output is fixedMMC1. Battery snapshot APIs are not a complete Studio save-game system. Projectiles/components/dialog/quests/inventory/checkpoints, global libraries, broad profiling and original-game recovery remain development work.

## Plan
### Reuse existing work

The compiler/linker, bank-aware source maps/downloads, world packer, sprite cycling, SFX arpeggios, native/WASM harnesses, CI and production CPU/core already exist. Extend their maintained paths; do not repeat historical repairs. Unchecked acceptance means evidence/signoff is pending, not absent implementation. For every candidate identify existing source/tests, classify implemented/pending acceptance/partial/missing, and implement only the demonstrated gap. The work audit is recorded in the development-record section; current file changes are documented in compact recovery notes.

### Remaining environment scope

These are extensions to existing tools, not instructions to rebuild their foundations. Current implementation and counts appear only in Current state. Completion criteria appear only in Acceptance and release.

| Area | Remaining capabilities |
|---|---|
| Project workflow | Genre templates, command/search palette, bookmarks/layouts, cross-editor undo/redo, autosave/recovery, snapshots/diffs, stable asset IDs, safe cross-editor rename/reference repair, portable project/asset packages and migrations; accessible onboarding and offline setup. |
| Code/build | Completion/signatures, symbol navigation/find references, source-aware rename, reusable modules/hooks, generated-source provenance, explicit custom-loop integration, debug/release profiles, incremental builds, CLI automation, allocation/segment/symbol reports and measured stack budgets; assembly interoperation through current linker. |
| Characters/combat/AI | Shared player/enemy projectiles, aim/spread/piercing/lifetimes/ownership, windup, hurt/attack boxes, knockback/stun/damage categories; reusable components/prefabs/placement overrides, idle/alert/attack/retreat/waypoint/boss/wave systems and persistent defeated state. Extend current melee/cooldown/pursuit/spawner workers. |
| Genre movement | Platformer gravity/jump/landing, one-way/moving platforms, slopes/ladders and jump buffering; top-down diagonals/dash/climb/swim, interaction facing and animation transitions. |
| Game systems | Parameterized event groups/functions, entity/global scopes, AND/OR groups, state machines and conflict diagnostics; dialog/portraits/choices/wrapping/localization; quests/journal/puzzles, inventory/items/equipment/shops/crafting; checkpoints/lives/continues/progression/save slots, HUD/menus/pause/cutscenes/fades and seeded encounters. |
| Art/assets | Global shared libraries/palette families, batch/select/stamp/import tools, sprite sheets/quantization previews, animation tags/events/onion skin/state transitions, asset repair, CHR layout/residency/streaming, fonts/localized text budgets and palette cycling/fades. Reuse existing paint/fill/flip/metatile/timeline/variant/dedup tools. |
| Worlds/cameras | Collision/property/object layers, regions/terrain brushes, room templates, connection/spawn repair, smooth cameras/dead zones/bounds/shake/scripts, seam/attribute streaming, parallax/supported status splits, triggers/hazards/interactive/destructible terrain, persistent map changes and seeded generation. |
| Music/SFX | Pattern/order/song-section workflows, reusable instruments/phrases/envelopes, keyboard entry/copy/paste/solo/mute/playhead, richer arbitration/transitions/adaptive cues and captures; Noise authoring across schema/UI/compiler/runtime while retaining legacy3=DMC. Existing tracker/SFX/arpeggios/WAV/DPCM remain foundations. |
| Emulator/hardware | Supported mapper/region/capability profiles, cartridge layout tools, controller remapping/physical acceptance, replay/save states/capture, explicit unsupported cases and independent emulator/hardware comparison. Improve measured fidelity in the existing core. |
| Debugging | Conditional/hit-count breakpoints/watchpoints/logpoints, disassembly/source stepping, stack/call history, typed watches/arrays/records, memory search/diff/diagnostic edits, live entity/event/mapper/APU/VRAM/timing views, traces/filter/export and bounded reverse stepping where feasible. |
| Profiling | Frame/routine/NMI/vblank attribution, PRG/CHR/segment pressure/dedup savings, RAM/stack/pools/save budgets, sprite counts/drops/flicker per scanline, streaming/audio costs, missed frames, thresholds and worst-case regression comparisons. Distinguish estimates from executed measurements. |
| Tests/releases | Editable recorded input/seeded replay, richer game-state/pixel/audio assertions, divergence/failure inspectors, parser/import/migration fuzzing, suite/CI reports; cartridge/header/vector/compatibility reports, reproducible source/project packages, licenses/manifests, offline distribution and supported external-emulator/flash-cartridge workflows. |
| Original game recovery | Later Phase48 track below: preserve original mapper/engine/uncertain bytes, exact reconstruction first, verified resources/semantic edits and independent compatibility evidence. |

### Delivery order

1. Finish cooldown/combined combat checks; then shared projectile/response pools.
2. Reusable components/event groups and explicit custom-code integration.
3. Persistent collection/inventory/checkpoints/save contracts and HUD.
4. Dialog/quests/menus in a complete small adventure.
5. Genre movement, collision layers and connected scrolling/cameras.
6. Workspace undo/recovery/reference/packaging tools alongside the authoring features that need them.
7. Art/audio production libraries/import/timeline/song workflows and Noise authoring.
8. Debugger/profiler expansion; move measurement tools earlier when budgets block a feature.
9. Replay/visual/audio/hardware/device acceptance and supported mapper/region matrices.
10. Clean-package release/offline tooling and documented samples.
11. Original cartridge recovery after authoring foundations are stable.

Each slice needs a short implementation plan, real production-ROM behavior, persistence/reload, combined-feature limit checks, required source/frontend verification and current recovery notes. User requests govern scope and publishing. Original hardware/release gates remain mandatory and do not disappear behind this product order.

### Original phase index

Original phase IDs are preserved; detailed criteria are centralized in [Acceptance and release](#acceptance-and-release). This index names work areas, not completion status. Earlier Completed headings were implementation labels and never replaced release signoff.

| Phase | Original work area |
|---|---|
| 1 | Foundation & Roadmap |
| 2 | String Data Support |
| 3 | Data Tables (DATA/READ) |
| 4 | Multi-File Projects |
| 5 | 16-bit Math Expansion |
| 6 | Advanced Math (Mul/Div/Signed) |
| 7 | Switch/Case Statement |
| 8 | Structs (Records) |
| 9 | Enums & Constants |
| 10 | Macros |
| 11 | Standard Library - Controller |
| 12 | Standard Library - Text Engine |
| 13 | Metasprite System |
| 14 | Animation Engine |
| 15 | Object Pooling |
| 16 | Collision - AABB |
| 17 | Collision - Point & Tile |
| 18 | Scrolling - Horizontal |
| 19 | Scrolling - Vertical |
| 20 | Random Number Generator |
| 21 | Audio - DPCM Support |
| 22 | Audio - SFX Priority |
| 23 | Audio - Envelope Editor UI |
| 24 | Audio - Arpeggios |
| 25a | Audio - SFX Engine Core |
| 25b | Audio - SFX Editor UI Foundation |
| 25c | Audio - Visual Envelope Designers |
| 25d | Audio - Import/Export & Full Integration |
| 26 | Visual - CHR Import |
| 27 | Visual - Metatile Editor |
| 28 | Visual - Metatile Integration |
| 29 | Visual - World Editor for MAPS |
| 30 | Visual - Sprite Editor |
| 31 | Emulator - WASM Integration |
| 32 | Emulator - UI Wrapper |
| 33 | Emulator - Input |
| 34 | Debugging - Protocol |
| 35 | Debugging - Source Maps |
| 36 | Debugging - Breakpoints |
| 37 | Debugging - Memory Viewer |
| 38 | Debugging - PPU Viewer |
| 39 | Repository Ingestion & Bus/Cartridge Architecture |
| 40 | Ricoh 2A03 (6502) CPU Core Hardening |
| 41 | Ricoh 2C02 PPU (Picture Processing Unit) |
| 42 | Core Memory Mappers |
| 43 | Ricoh 2A03 APU (Audio Processing Unit) |
| 44 | The "Swiss Army" Debugging Suite & Frontend |
| 45 | Interactive Tutorials |
| 46 | Asset Library |
| 47 | Desktop Build |
| 48 | ROM Import and Editable Project Recovery |

### Phase48 recovery detail

Planned original1980s NES/Famicom ROM import; the existing cartridge loader does not implement editable-project recovery. Keep this later scope in the same plan.

#### Recovery boundary

Import original 1980s NES/Famicom cartridge games into complete editable projects: code, graphics/palettes/maps/animation, game flow, music and SFX. Save/reopen, preview, rebuild and export a playable ROM while preserving the original engine, mapper/bank layout, metadata, trainer, RAM/battery, timing and expansion requirements.

Begin with cartridge .nes and regional variants, staging mapper families0/NROM,1/MMC1,2/UxROM,3/CNROM,4/MMC3,7/AxROM after individual fixtures pass. Era alone does not identify hardware/asset formats. Famicom Disk System requires a separate disk path. [NES2.0 format](https://www.nesdev.org/wiki/NES_2.0) · [submappers](https://www.nesdev.org/wiki/NES_2.0_submappers).

Compiled games usually lack source names/comments/types. Useful recovered representations need provenance and manual annotations; exact original-source identity and automatic recovery of all semantics cannot be promised. Preserve unknown bytes/fields instead of silently replacing engines/codecs with Studio presets. Compatibility means a documented matrix of independent emulators supporting the cartridge, not every existing/future emulator.

#### Recovery stages

| Step | Build | Required evidence |
|---|---|---|
|1. Project format | Versioned recovered mode; ROM hash/hardware/banks/physical offsets/labels/annotations/resources/analysis status; compatible authored projects | Reopen intact; original input/build ROM remain local/ignored; build uses recovered source/assets |
|2. Inspect/extract | Validate lengths/hardware before transactional PRG/CHR/section extraction; assign every byte an owner | Failure preserves existing project/running game; uncertain regions remain tracked |
|3. Code/data recovery | Vector-led bank-aware disassembly plus bounded production-core traces; separate CPU addresses/physical banks | Retain indirect/bank/interrupt paths, RAM code, overlapping interpretations and undocumented opcodes; traces prove observed paths only |
|4. Exact reconstruction | Distinct build mode preserving original layout/mapper/alignment/raw instructions/data/header; current authored compiler stays fixedMMC1 | Import→save→reload→source-only build reproduces every untouched byte without original input; no embedded-ROM shortcut |
|5. Behavioral baseline | Controller scenarios, reset/interrupt/bank events, screen/audio/save observations for games/regions | Original/rebuilt menus/gameplay/transitions/late paths agree; boot alone insufficient |
|6. Graphics/world codecs | RawCHR first, then verified palettes/sprites/animations/maps/collision, CHR-RAM/compression profiles | Unchanged decode/encode round trip plus visual edit; retain unknown fields/storage limits |
|7. Original audio drivers | Verified driver profiles for songs/instruments/sequences/SFX/DPCM and supported expansion audio | Music/SFX edits rebuild in original driver; preserve timing/channel behavior; unknown drivers retain editable code/data |
|8. Studio game systems | Cross-references/functions/variables/scenes/events and optional supported SANBasic conversion | Every item links to bytes/evidence; preserve timing-sensitive assembly; reject conflicting shared/overlapping edits; demonstrate gameplay/code edits |
|9. Export/compatibility | Pin independent emulator versions/configurations and hardware support; export source/ROM/test/recovery reports | Unchanged/edited boot/input/render/audio/interrupt/save tests; native+WASM shared core count as one implementation |
|10. Expand support | Per-game/mapper catalog separates exact rebuild, decoded resources, semantic recovery and emulator checks | Banking/compression/hardware/formats expand only through fixtures; partial recovery remains explicit |

Completion requires representative supported 1980s games, exact untouched source-only rebuilds, persistent project/provenance, working code/art/map/music/SFX edits, independent-emulator results and the full in-app import/analyze/edit/preview/build/export workflow. A raw byte dump alone does not satisfy Studio integration. Unsupported capabilities must fail clearly without overwrites. This later milestone does not sign off phase 40 or the unfinished game platform.


## Studio authoring

Studio integrates scenes/assets, reusable entities and enemy spawners, shared color/facing variants, configurable hitboxes, optional melee combat, variables/events and controller tests. It generates SANBasic; existing `.swiss` source files remain compatible. [Resume and evidence](#current-state) · [remaining work](#plan).

### Authoring workflow

1. Run the server and Create playable starter with a unique lowercase project name. It supplies original editable code, art/maps, animations, music/SFX and Title→Meadow↔Ruins. Existing projects are preserved.
2. Select a scene in the graph; assign its populated World cell, spawn, movement and entry audio. Transitions use their displayed order: first matching button/timer wins. Renaming retains IDs; removing a scene repairs start/incoming transitions, while door/event/test references remain for explicit repair.
3. Edit assets in Graphics/Map/World/Sprites/Music/SFX. Reusable entities select an animation/behavior; Place entity adds a scene instance. Select a marker, click to snap at 8 pixels or enter exact X/Y. Set door target/patrol endpoint. Preview uses real CHR/palettes and actual collision outlines; P marks player spawn. Player/entity offsets and dimensions fit within their 16×16 footprints.
4. Choose Enemy to enable combat; set enemy health/contact damage and movement. Stationary / patrol uses an optional endpoint (blank is stationary). Pursue player horizontally requires a right endpoint for every enemy or spawner placement, plus detection range. Configure player health, attack A/B, damage/reach, direction, immunity frames and defeat scene. All directions retains the radial attack; Face left / right strikes the current horizontal front. Remove a scene transition using your attack button if it takes priority. Add a pressed-START Restart game event in the defeat scene to restore the starting state. Combat is optional; the starter keeps its original controls.
5. Add game variables, then scene events with entry/update/button/timer triggers, AND conditions and ordered actions. Move events/actions earlier/later for priority. Variables persist between rooms; arithmetic clamps 0..65535. Scene change or restart must be last and stops further rules that update. Health/defeats can drive win/lose rules.
6. Save design stores live editor snapshots and metadata. Preview flow compiles its own harness; custom main.swiss stays intact. Starter controls: START enters gameplay, arrows move, B changes rooms, SELECT returns home; contact collects a coin and fresh A inside a doorway enters.
7. Inspect generated code opens virtual read-only game_flow.swiss with source-map provenance. Put custom systems in main.swiss or another include. Standard Compile/Run uses your code; Studio preview/tests use the flow harness.
8. Smoke test executes 120 no-input frames. Controller scenarios save the design, boot a fresh ROM for 10 frames, hold/release buttons for each duration and assert state, including health/defeats when combat is enabled. Blank expected fields omit assertions;0 asserts zero. The first mismatch stops the run and displays actual state/cycles.

Example rule: Score=0, Meadow/update, pickup count≥1, once per visit, addScore=100. It awards once after any pickup; because the counter persists, returning awards again without another coin. Add Score=0 as a condition for a whole-game one-time reward. Another pressed-A rule with Score≥100 can enter Ruins while preserving score. Custom code accesses gv_Score; rename source symbols in custom code if you rename variables.

#### Reusable enemy waves

Create an Enemy definition first. Add another definition, choose Enemy spawner and select its target enemy. Set Max alive1..8, interval1..65535 updates, first delay0..65535 and total per visit0..65535 (0 unlimited). Place the emitter at its spawn point; its movement endpoint is optional for patrol and required for pursuing targets. Its editor-only marker previews target art/collision. The generated pool reserves slots/OAM even while empty; its enemies inherit the target's health, damage, motion/range, animation and visual variant. Removing a referenced enemy definition is blocked; missing/changed targets stay visible for repair.

Each eligible update decrements the timer and spawns at0 if a slot is free. Delay0 starts on the first eligible update; delayN waits until the Nth eligible update. A full pool waits at0 without consuming quota. One enemy spawns per emitter/update; an interval starts after a successful spawn. Enemy interaction runs before spawning, so recycled newborns cannot inherit that update's attack. Reused slots reset position/direction/health/animation. Room entry resets pools/timers/quotas; death freezes spawning, and transitions suppress destination spawning until the next update. Shared behavior routines prevent duplicating combat/patrol code per pool slot.

Conditions can read a spawner's count in the event's own scene. To end a wave after all enemies are defeated, combine a finite spawn quota with a defeat-count condition appropriate to your game's persistent counter. Saved steps assert counts by emitter instance ID; choose an expected scene (or the selected scene) to show its spawner fields. Missing/out-of-room assertions fail, including expected0.

#### Shared color and facing variants

Player/entity palette, horizontal/vertical mirror and automatic horizontal facing reuse the original animation/metasprite/CHR data. Create color variant copies only a definition and chooses an unused SP0–SP3 palette. Change one or two of its three visible colors; other copied colors stay intact. Existing occupied palettes are never automatically overwritten. Manual palette edits affect every actor using that palette. All four slots occupied requires choosing/releasing a palette explicitly. Spawners inherit their enemy's visual settings.

Mirror applies to the entire frame's signed-offset bounding box: positions are swapped and hardware flip bits toggled. Original priority and tile palettes remain intact unless an explicit palette overrides them. Facing toggles horizontal mirroring relative to the selected base mirror; configure the base to face right. Player facing persists between directional inputs and resets on entry; entities use their patrol direction. Collision remains authored separately. Identical metasprite records and animation sequences are automatically stored once; controller-test results display authored/unique counts and actual metadata bytes saved. [Compiler drawing/reuse contract](#shared-sprite-and-animation-data).

Connect current project enables metadata without replacing custom source. Place maps in World and resolve resource references before saving. Integrate generated flow as follows:

```basic
INCLUDE "game_flow.swiss"
SUB Main()
    GameFlowStart()
    DO
        WAIT_VBLANK
        GameFlowUpdate()
        ' Custom logic uses the controller state already read by the flow.
    LOOP WHILE 1
END SUB
```

### Saved schema and limits

Metadata.design is optional; legacy designs default to empty collections. Version 1 uses these fields:

| Object | Fields and validation |
|---|---|
| Design | version=1, title1..120 bytes, core_loop≤4000 bytes, start_scene, optional animation, scenes, optional entities/variables/events/tests/combat, player_hitbox (default16×16), optional player_visual |
| Scene | id, name, map_col/map_row, spawn_x/spawn_y, movement, optional music/sfx, transitions, optional instances. Up to255 scenes; IDs unique1..64 bytes; names1..28 ASCII letters/digits/spaces or -_!?. Spawn X0..240/Y0..224. |
| Transition | target scene ID, trigger=a/b/start/select/up/down/left/right/timer, frames. Up to16 per scene, unique triggers; timer1..65535 frames. |
| Entity definition | id/name/animation/behavior/hitbox/enemy/visual/spawner;≤32 definitions, unique1..64-byte IDs, name1..120 bytes; behavior=decoration/pickup/door/patrol/enemy/spawner. Non-spawners need a valid animation. enemy={health,damage,motion,detection_range}; health/damage1..255, default1/1; motion=patrol (default) or pursue; detection_range default64,1..255 when pursuing. Enemies require combat. |
| Spawner settings | entity=existing Enemy definition ID, capacity1..8, interval_frames1..65535, initial_delay_frames0..65535 (default0), total0..65535 (default0 unlimited). Recursive/non-enemy targets are rejected. |
| Visual settings | palette=null (preserve frame palettes) or0..3; flip_x/flip_y/face_movement booleans, defaultfalse. Same fields for player_visual and entity.visual. |
| Hitbox | x/y offsets0..15, width/height1..16; offset+dimension≤16 per axis. Default={x:0,y:0,width:16,height:16}. Art origin/extents are unchanged. |
| Combat | health/invulnerability_frames/attack_damage1..255, attack_button=a/b, attack_reach0..32, death_scene existing ID; attack_mode=all_directions (default) or facing; attack_cooldown_frames0..255 (default0/off). Optional; absent means no combat state. |
| Instance | globally unique id, entity ID, x/y, optional target_scene/patrol_end_x;≤16 runtime slots per scene including authored emitter markers plus reserved pool slots. Origins X0..240/Y0..224. Doors need existing target; movement X<endpoint≤240. Enemy/spawner endpoints are optional for patrol, required for pursue targets. Generated pool IDs=@spawn:instanceID:index; collisions with authored IDs reject saves. |
| Variable | id/name/initial;≤32, unique1..64-byte IDs; name1..32 letters/digits/underscores starting with a letter, unique ignoring case. initial unsigned 0..65535; generated gv_Name WORD. |
| Event | id/name/scene/trigger/once_per_visit/conditions/actions;≤32, unique1..64-byte IDs, name1..120 bytes, existing scene.≤8 conditions,1..8 actions. |
| Saved test | id/name/steps;≤32 tests, unique1..64-byte IDs, name1..120 bytes,1..64 steps. |
| Step | positive frames, unique controller buttons, optional expected_scene/expected_x/expected_y/expected_pickups/expected_active_entities/expected_health/expected_defeated; expected_variables:{variableID:value}, expected_spawns:{emitterInstanceID:count} (≤16). Health0..255; defeats/counts0..65535. Total≤600 frames including10 boot. Timer is not a controller button. |

World cells must be populated; referenced assets must exist. Missing/deleted references are visible and invalid saves are rejected before file writes. Instance/definition selections remain linked; removing a used definition/variable is blocked until its instance/event references are removed. Missing scene/variable test expectations remain coverage and fail rather than disappearing.

#### Event forms

- Trigger: `{kind:"enter"}`, `{kind:"update"}`, `{kind:"pressed",button:"a"}`, `{kind:"held",button:"a"}` or `{kind:"timer",frames:120}`. All eight controller buttons are supported.
- Condition: `{source,comparison,value}`. Source is `{kind:"variable",variable:ID}`, `{kind:"spawned",instance:emitterID}` in the event's own scene, or kind pickups/x/y/ticks/health/defeated. Operators: equal/not_equal/less/less_equal/greater/greater_equal, against unsigned0..65535. All conditions are ANDed. Pickup count is0 without slots; health/defeated conditions evaluate0 without combat.
- Actions: kind set/add/subtract with variable/value; scene with scene ID; music with track ID; sfx with effect ID; stop_music; restart. Values are unsigned0..65535. Add saturates at65535, subtract at0. Scene change/restart must be final; audio uses existing arbitration and destination entry audio may replace it. Restart calls GameFlowStart, resetting variables, pickups, combat and room state.

### Runtime contract

GameFlowStart resets variables/counter and enters the start scene. Entry disables rendering/NMI for map loading, sets spawn/movement/audio, resets timer/player animation and room slots, then restores rendering. GameFlowUpdate reads the controller once, advances a saturating WORD timer, applies transitions, moves the player, processes entities/events, then draws/scrolls. Nonzero map tiles are solid.

Pickups deactivate on contact with the movable player, increment a saturating counter and respawn on every room entry. This is room-local collection, not persistent inventory. Doors require overlap and fresh A; transitions have priority and guards prevent same-update cascades. Patrols/moving enemies move one pixel/update, turning at walls/endpoints. Player/patrol tile probes and object contact use configured collision boxes; touching edges do not overlap. Leading-edge corners plus midpoint prevent unaligned boxes skipping an interior wall tile. Entry clears shared slots and initializes positions/directions/animations/enemy health; active entities render even without player animation.

Combat attacks use a fresh press. Default all_directions expands the player's box by reach on every side, including diagonals. Facing uses the front half of the player's hitbox plus horizontal reach with strict vertical overlap; odd widths round outward, touching vertical edges do not hit. Left/Right inputs set gf_player_facing even when blocked by a wall, Right wins simultaneous horizontal inputs, release retains facing and room entry resets it right. Facing works without a player animation. Enable player Face horizontal movement to mirror the art as well; base art should face right after its authored mirror. Each eligible enemy receives attack_damage once per accepted swing; held input does not repeat. Enemy health clamps at0, defeated objects deactivate before contact processing, and the WORD defeat counter saturates at65535. Contact damage then clamps player health at0 and starts immunity; only one overlapping enemy can damage per eligible update. Immunity counts down once per update, across room changes. Health/defeats persist between rooms; enemies respawn. Zero health enters death_scene, hides the player and disables movement until GameFlowStart/restart. Scene transitions and earlier door changes suppress old-room combat. Custom code can use gf_health/gf_invulnerable/gf_defeated; combat allocates no state when absent. Projectiles, vertical melee, knockback and richer AI remain planned.

Optional attack cooldown decrements once per GameFlowUpdate, before accepting a swing. ValueN permits the next swing Nupdates after the previous one;0 keeps legacy fresh-press behavior without extra state. Presses while the timer is nonzero are discarded rather than queued, and holding through expiry never repeats. One accepted swing is latched before entity dispatch so all placed/spawned targets share it; an empty swing still spends cooldown. Room transitions take priority and cannot spend an attack, room entry clears the transient latch but preserves the timer, and restart clears both. Only a movable living player can start a swing. Enabled cooldown appends gf_attack_cooldown/gf_attacking after every previous declaration; custom code uses names rather than harness offsets.

Pursuit moves one pixel/update horizontally toward the player's origin when the absolute origin distance is at most detection_range on both axes. It stays at the authored Y, stops at equal X, waits outside range and requires a movable living player. It stops at walls and inside the inclusive span [instance.x,patrol_end_x], facing the intended direction while blocked. Detection/attack comparisons avoid unsigned subtraction near screen edges. Pursuit reuses the existing direction byte and adds no enemy RAM or art. Spawner pools inherit target motion/range and their emitter's span/Y; each emitter shares one worker routine. Default patrol/stationary behavior is unchanged.

Events run in metadata order and actions sequentially; every matching event can fire until a room change suppresses dispatch. Destination entry rules run on the next eligible update instead of recursively while loading maps. Entry conditions receive one evaluation per visit; false conditions do not retry. Multiple entry rules share the flag. Once-per-visit latches set only after trigger/all conditions match and reset on entry. Timer events compare≥threshold: repeated timers can run every eligible update after it; editor defaults timer triggers to once per visit. Conditions can become true later. An authored entry cycle may change rooms every update, but cannot recursively overflow the stack.

#### Resources and source ownership

RAM estimate:9 base +5 optional player animation; with slots,3 counter/guard +9×largest-room runtime slots (including pools); +2×variables; events,+1 entry flag +1×events +1 guard if no slots; combat,+4 health/immunity/defeats +1 health byte/slot; spawners,+4×largest-room emitter count +5 shared worker bytes; animated automatic player facing or facing melee,+1 shared byte; enabled cooldown,+2last bytes. Existing combat/spawner/facing offsets stay stable. Maximum16 slots/player/32 variables/32 events/combat/8 emitters/facing/cooldown is318 of560 variable bytes before custom code (316with cooldownoff). Pursuit, entity facing and palette/mirror choices add no state; unused definitions allocate no slots.

OAM estimate sums largest-frame tiles for every placed actor and every reserved pool slot plus player, rejecting>64; editor-only emitters do not render. Color/facing variants reuse tiles but still consume normal OAM when drawn. Eight sprites/scanline, CPU/vblank and actual ROM/custom RAM limits remain separate profiling/compiler concerns.

The virtual flow include is regenerated from validated saved metadata/assets. Writing/deleting it is rejected. Enabling Studio rejects a preexisting physical game_flow.swiss, preserving it for user-directed repair. Immediate input capture makes Save/Preview independent of blur; obsolete async results are discarded. Source maps retain generated-file provenance.

#### Test API

POST /api/projects/:name/flow/test accepts frames1..600 (default120) or test_id. The server compiles one owned design/assets snapshot, drains audio between frames and reports passed/message, final/per-step scene/X/Y/pickups/active_entities/entities/variables/health/invulnerable/defeated/generated_ram_bytes, frame count, ROM bytes and CPU cycles. Enemy observations include health; non-enemies/no combat have null health. Combat fields are null without combat, so retained health/defeat assertions fail instead of disappearing. Variables are keyed by stable ID and displayed by current name. Assertion failures return HTTP200/passed:false; malformed requests400.

Expanded entity observations include runtime slot, generated ID and spawned_by. spawners is keyed by emitter ID and reports spawned/remaining_frames/active/capacity/target entity. asset_reuse reports compiler-authored/unique metasprite/animation counts and metadata sizes/savings. Spawn counters saturate65535 while unlimited emitters continue to reuse slots. Counts/timers are local to the current room; missing assertions remain failures.

Each scenario boots fresh for 10 frames. Adjacent identical button holds stay held; empty steps release. Fixed offsets5C0 scene/5C1 ticks/5C3x/5C5y/5C7movable/5C8loaded apply only to isolated MAIN_SOURCE; entity/variable state follows declaration order. Custom code uses names, not these addresses. These tests exclude custom main, physical controls, fidelity and full release acceptance; [evidence](#development-record) records actual browser checks.


## Runtime contracts

### Core ownership and supported boundary

Native tests and generatedWASM use checksum-pinned TetaNES0.12.2 plus patches/tetanes-core-0.12.2.patch reconstructed by scripts/prepare-core.mjs. Do not create a second validationCPU or patch the Cargo registry. The project parser validates metadata/payload before backend allocation; each PRG/CHR region is limited to64MiB. Whole-bank exponent sizes adapt to backend linear counts; partial-bank layouts and unsupported console/timing/expansion/submapper/geometry combinations fail explicitly. A trainer is removed from the backend copy then verified through the real bus at7000–71FF; original caller bytes remain unchanged. Failed loads retain the old game and hardware signals.

NROM rejects nonzero submappers/four-screen/nonstandard expansion and unsupported RAM geometry; mixedCHR ROM/NVRAM and backend-declaration mismatches fail. Named PRG/CHR/IRQ accessors reach actual CPU/PPU bus dispatch; MMC3 IRQ/four-screen3000mirrors have production regressions. Parallel independent emulator execution is unsupported because backend event signals are thread-local; browser owns one active instance. Physical PRG debug offsets exclude headers, whereas compiler SourceMap.rom_offset is file-relative including the16byte header for generated trainer-free images.

Generated MMC1 switching uses07F0desired bank/07F2retry flag; reset plus all five serialPRG bits repeat after interrupted attempts. Epilogues preserve07F1return scratch. Fixed-upper-bank/generated-write assumptions remain; inline assembly must coordinate separately. Dynamic NMI/IRQ binding uses one-store selectors07F8/07F9 (zero=startup; immutable fixed-bank tables for others), including banked-handler trampolines.

Interrupt scratch00–0F,14–17,F0–F7 plus07F1/registers is stack-preserved; persistent controller/text/sprite/scroll/RNG state is not rolled back. IdleNMI includingDMA measured1136cycles. Under three looping tonalSFX+DMC, built-inPPU writes finished964–1324cycles before2273NTSCvblank; entire handler4294–5204cycles within one frame. User handlers budget their own PPU work. Rejected loads restore IRQ/NMI/DMA masks/halt/dummy/OAM state. These bounded measurements do not complete broad hardware signoff.
The implementation uses the same checksum-pinned TetaNES 0.12.2 source and project patch in native and generated WASM builds. Run `node scripts/verify.mjs` for the complete local matrix. The native CI job runs Windows, macOS, and Linux; a configured matrix is not evidence that a remote run has passed.

### CPU/debugger interface

`trace_instruction()` returns a pre-instruction snapshot and executes one production-core instruction. `peek_cpu()` and RAM/PPU debug inspection do not acknowledge hardware registers. Physical PRG identity comes from the executing mapper, in 16 KiB units. `CpuState.cycles` is a `u64`, exported as JavaScript `BigInt`; never coerce it to a 32-bit integer. Free wasm-bindgen snapshot objects after copying their fields.

`viewport_position()` returns `[x,y]` in the logical 512×480 nametable space. It decodes the PPU's programmed temporary address and fine X without reading status or changing the write latch. It describes the current programmed origin, not a raster-effects history.

`emulator/tests/cpu_contract.rs` inventories all 256 dispatch entries and the 151 official encodings/56 mnemonics. The canonical trace exercises 149 official encodings; BRK and CLI have separate execution tests. The suite exhaustively verifies ADC/SBC over all byte operand pairs, carry inputs, and D states, plus page penalties, zero-page wrapping and all 12 JAM error/reset paths. Separate interrupt/trace tests cover stack-status serialization, IRQ masking/polling, NMI, BRK/RTI, and OAM DMA. Supplemental timing evidence is recorded in the acceptance worksheet; do not infer every bus timing edge from opcode coverage alone.

### Cartridge battery layout

The loader commits a replacement only after metadata, backend allocation, and trainer validation succeed. NES 2.0 allocations that differ from declared capacity are rejected explicitly. Unsupported combinations of CHR ROM with nonvolatile CHR RAM are rejected; partial-bank exponent geometries remain unsupported.

Volatile PRG RAM precedes PRG NVRAM in the backend allocation; volatile CHR RAM precedes CHR NVRAM. `save_ram_snapshot()` returns **PRG NVRAM followed by CHR NVRAM**, excluding volatile bytes. `restore_save_ram(bytes)` validates the complete payload length before writing either region. The caller owns storage and association with the right ROM; equal size alone does not identify a cartridge. A hard reset reinitializes volatile state, restores declared battery regions, and reapplies original trainer boot data. Loading another ROM starts another battery owner; a failed load retains the current owner. Legacy iNES battery headers use their normalized PRG battery capacity.

### Fixed-bank helper placement

Color/facing variants use additive Sprite.DrawVariant/Animation.DrawVariant, shared frame data and exact-content metadata aliases. Legacy draw paths stay fast; [sprite reuse](#shared-sprite-and-animation-data) defines the two-byte bounds footer, palette/flip semantics, interrupt scratch and measured savings. Studio spawners reuse bounded slots and shared enemy routines; [Studio contracts](#studio-authoring) define quotas/state/phase ordering.

The compiler retains runtime instructions reachable from startup, guest code in every bank, interrupts, symbolic inline-assembly operands, and transitive helper references. Literal data survives pruning. The assembler then resolves final addresses and checks segment overlap. Helper CPU addresses are not a stable public ABI; use named labels for inline assembly. This leaves space before fixed audio/asset segments without silently relocating or overwriting them.

Dynamic assignment uses the destination type's storage width. Assigning a WORD expression to a BYTE array writes one byte; assigning BYTE to WORD initializes the high byte. `STR` renders WORD unsigned and INT/negated expressions signed; `VAL` and `STR` preserve their cursors across arithmetic helpers.

### Sound and SFX data

Use `Sound.Play(track_id)` for music, `PLAY_SFX effect_id` for effects, and `Sound.Stop()` for hardware/software silence. Invalid or missing runtime IDs leave an active channel alone. Track channels are 0=Pulse1, 1=Pulse2, 2=Triangle, 3=DMC. Macro SFX use tonal channels 0–2. Lower priority cannot replace higher priority; equal priority retriggers/replaces. An effect pauses the displaced music timer and restores its note/envelopes on completion.

Older SFX JSON loads without `arpeggio_sequence`, defaulting to `[]`. Effects contain name/channel/priority/speed/does_loop plus volume (`u8`), pitch (`i8`), duty (`u8`), and arpeggio (`i8`) sequences. Volume is 0–15, duty 0–3; each sequence has at most 127 steps. Pitch is an independent timer offset; arpeggio is signed semitones from macro base **note 48**. The historical 96-entry period table is preserved; its old octave labels must not be used as a physical-frequency guarantee.

Music at `$D100` and effects at `$D900` start with a count and little-endian absolute pointers. An effect record is six bytes: channel, priority, volume-envelope ID, pitch-envelope ID, duty-envelope ID, arpeggio-envelope ID. IDs are assigned after user envelopes. Envelopes at `$DA00` have count/pointers, then a loop index (`$FF` for no loop), `[value,duration]` steps and a zero-duration terminator. Speed zero means one frame. Finite sequences hold their final values until the longest sequence completes; each loop wraps independently. Music notes are `[duration,pitch]`, with pitch `$FF` as rest and duration zero terminating the track. Empty tracks terminate immediately.

DPCM tables at `$D480` store address/length pairs. Sample data begins at `$E040`, is 64-byte aligned, and is padded to `16*n+1` bytes. Individual samples support at most 4081 bytes and are rejected rather than truncated. A DMC track selects the authored sample and rate, independently of the tonal period table. Count and segment capacity checks apply before injection.

Music-editor Play compiles the selected track through the production APU runtime. Notes serialize as numeric values; DMC rows select sample IDs with 16 rate choices. WAV import resamples to the selected NTSC DMC bit clock, rejects oversized/invalid input, and preserves the current project if an asynchronous import becomes stale. Removing a sample or envelope repairs remaining track references. The tracker displays the physical octave of the preserved timer table: index 48 is C6.

### World maps, uploads and gameplay

`World.Load(screenColumn, screenRow)` returns `$FF` on success and zero for an empty/out-of-bounds cell. It loads all 960 tiles and 64 attributes with rendering disabled, then restores mapper bank, PPU control/mask, fine scroll and caller IRQ mask. A failed lookup preserves the selected map. `World.Tile(localPixelX, localPixelY)` and `Collision.Tile` read that selected map and reject coordinates outside 256×240. Legacy one-map projects retain the `$D500` image.

World assets use linker-checked data in unused MMC1 banks 1–6. Directory version 1 starts with `[version,width,height,stride=5]`; each cell has `[bank,addressLow,addressHigh,lengthLow,lengthHigh]`, with bank `$FF` for empty and length 1024 for a map. Maps are 1024-byte aligned and deduplicated by authored map index. The directory supports at most 3276 cells; actual map capacity depends on free banks and directory padding. RAM `$07FA-$07FC` stores the selected map; zero-page `$F9` shadows PPUMASK.

`Scroll.LoadRow(y, tiles, optionalAttributes)` and `LoadColumn(x, tiles, optionalAttributes)` accept BYTE arrays with at least 32/30 tiles and, when supplied, 8 fully composed attribute bytes. The queue uses at most 46 bytes within `$0380-$03BF`: four header bytes, tile data at offset 4, attribute address at 36 and attribute data at 38. Tile and attribute phases run on consecutive NMIs; busy requests are skipped. Wait until `PEEK($0380)=0` before exposing a new seam. `$FD` advances after the generated NMI audio update and is the completed-frame signal used by WAIT_VBLANK when NMI is enabled.

Built-in PPU work precedes audio so active music/effects do not delay VRAM/scroll writes past vblank. Pool allocation reports its full sentinel, clears reused slots, and validates despawn indices without writing neighboring RAM. The [editable platform demo](../examples/platform-demo/README.md) exercises maps, includes, graphics, animation, input, wall collision, audio and pool reuse together.

### Frontend ownership

Studio generates protected ordinary source from validated metadata; preview/tests compile an owned design/assets snapshot in the same production core and preserve custom main. Scene/entity/event schema, RAM/OAM limits, deferred entry/visit semantics and variable assertions are documented once in [Studio contracts](#studio-authoring). The bounded harness excludes custom-main/device/fidelity acceptance. [Checkpoint/evidence](#current-state).

First Run/music preview shares one WASM initialization. Constructor and memory exports publish together only after validation; failed initialization permits a later retry. Only the latest Run for the same source/project/file context may compile. Closing the emulator or reloading its project invalidates pending startup/compile results and suppresses obsolete errors. Static IDE assets use `Cache-Control: no-cache`, including conditional responses, so unversioned JS/WASM is revalidated on subsequent requests. The PPU debugger is layered above the emulator overlay.

Full scaling fits the canvas into the remaining overlay height after the toolbar, including wrapped controls. The 1x and 2x modes retain intrinsic 256×240 and 512×480 content sizes. Full preserves the displayed image's aspect ratio and keeps Pause/Reset/Close within the viewport.

Project saves serialize and retain source/assets/project identity for each request; failed saves abort compilation. A later edit or project switch invalidates pending compile responses. One keyboard binding owns an emulator session; keyboard and gamepad holds combine, and both Shift keys can hold Select independently. Pause/reset/reload cancels queued audio and rebases scheduling. Hidden/closed sessions release inputs. Source maps preserve file/bank identity and reject edited snapshots. PPU views validate lengths and recreate memory views after growth.

The automated frontend harness verifies these workflows and graphics/SFX authoring round trips. Actual browser rendering, audible output, and physical controller observations require separately recorded evidence.


## Shared sprite and animation data

```basic
Animation.Play(actor, Walk)
Animation.DrawVariant(100, 80, actor, 1, $40)
Sprite.DrawVariant(100, 80, Body, 2, $C0)
```

DrawVariant arguments are x,y,AnimState/metasprite,palette,flip flags. Palette0..3 selects SP0–SP3;255 preserves each tile's palette. Flags$40 mirror horizontally,$80 vertically,$C0 both. Other flag bits are ignored. Original priority is preserved and original flip bits are XORed. Coordinates keep existing byte-wrap behavior. Whole-frame mirroring swaps tile positions around signed-offset frame bounds, then toggles tile flip bits; it also supports negative offsets and64-tile records crossing a page. This mirrors existing artwork; different front/back artwork still needs authored poses. Collision geometry remains independently authored.

Each metasprite keeps its legacy count followed by4-byte x/y/tile/attribute records. A two-byte footer stores minX+maxX and minY+maxY modulo256 for mirrored positioning. The default Draw helper remains unchanged and ignores this footer. Styled draws use a separate helper, stack-stage arguments before assigning scratch, and use interrupt-protected scratch00–0F. Animation states stay5bytes; variants share frame tables and maintain independent playback state. Entity automatic facing reuses patrol direction; player facing adds1byte.

The compiler aliases exact-identical metasprite payloads to one ROM address. Animation sequences also share data when their loop flag, canonical frame references and durations match. All public names/frame labels remain valid. It preserves source CHR placement, tile IDs, unique artwork and different timings. Metadata savings are measured before/after deduplication in the current footer format, excluding code/audio/maps/CHR and fixed ROM padding. Example production test: two identical two-tile metasprites plus two identical two-frame animations use19 instead of38 metadata bytes. Distinct color/facing choices add0 CHR or frame-table bytes; they require draw instructions and normal OAM capacity.

Linked source maps and Studio test responses contain asset_reuse: authored/unique metasprites and animations, metadata_before_dedup, metadata_after_dedup and metadata_bytes_saved. Names still map to the shared physical data. [Production regressions](../tests/sprite_reuse_execution_test.rs) prove OAM geometry/palette/priority, default-call isolation, shared addresses, unchanged CHR,64-tile bounds/limit and invalid-call diagnostics. [Studio usage](#studio-authoring) explains palette allocation and spawn budgets. Four sprite palettes and eight sprites per scanline remain NES hardware limits.


## Acceptance and release

This section owns acceptance criteria and signoff. [DESIGN](#plan) owns the plan, [resume](#current-state) owns current results, [runtime](#runtime-contracts) and [Studio](#studio-authoring) own implementation contracts, and [development record](#development-record) owns historical observations. Keep criteria here rather than duplicating handoffs/checklists.

Unchecked requirements mean pending acceptance, not missing code. Review existing modules/tests before repairs. Preserve passing compiler/core/editor implementations and add only missing evidence or reproduced fixes. Signoff identifies source revision, core/fixture revisions, commands/results, host/browser/device, evidence and reviewer/date. Current uncommitted work has no complete release signoff.

The original foundation, Phase1–40 and P39/P40 criteria below are preserved from the worksheet, including its two focused checked entries. Existing local/CI/browser evidence is in resume/record; no additional gate is marked complete by document merging.
### Foundation gates

- [ ] M0: baseline recaptured against the developer's actual HEAD; current roadmap numbering reconciled.
- [ ] R01: native host matrix, workspace checks, pinned WASM generation, browser harness, clean setup.
- [ ] R02: previously failing API fixtures now compile and boot; production/legacy helpers unified.
- [ ] R03: bank windows, reserved RAM, asset lengths, overflow, vectors, and deterministic output verified.
- [ ] R04: versioned file/bank/range source map and all consumers agree with actual assembly.
- [ ] R05: headless and browser harnesses execute the production backend with bounded runs.

### Per-phase acceptance

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

### Phase 39 detailed gate

- [ ] P39-01: actual core ownership and maintained patch strategy documented.
- [ ] P39-02: normalized metadata and parse/unsupported errors tested.
- [ ] P39-03: iNES sizes, trainer, CHR RAM, mirroring, malformed/truncated input.
- [ ] P39-04: NES 2.0 linear/exponent sizes, extended mapper/submapper, RAM/NVRAM.
- [ ] P39-05: named Mapper operations connected to actual CPU/PPU dispatch, proven by integrated routing tests.
- [x] P39-06: complete 16-bit bus boundary/mirror/side-effect/peek tests.
- [ ] P39-07: NROM fixtures, existing mapper-1 games, transactional load/reset, native/WASM acceptance.

### Phase 40 detailed gate

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

### Final release gate

- [ ] Integrated authored demonstration in handoff section 11 completed.
- [ ] Clean checkout reproduces all published commands and required generated artifacts.
- [ ] No required phase feature is merely deferred to a later-phase backlog.
- [ ] README, DESIGN, AGENTS Brain, architecture, schemas, support matrix, coverage, and fixture notices updated.
- [ ] Evidence artifacts identify the same implementation commit and core revision.
- [ ] Windows, macOS, Linux, browser, audio, and physical controller evidence attached.
- [ ] Reviewer signs off that phases 1–40 (including 25a–25d) meet their criteria.

### Later phase/product gates

| Phase / extension | Required proof |
|---|---|
|41 PPU | Supported PPU register/latch/scroll/timing, mapped background/sprites/priority/scanline limits and diagnostic fixture pixels through production rendering. Core existence alone is not a fidelity pass. |
|42 mappers | Supported NROM/MMC1/UxROM/MMC3 games/fixtures boot/play/reset with correct PRG/CHR/mirroring/IRQ/save behavior; publish tested board/submapper/region limits. |
|43 APU | Production pulse/triangle/noise/DMC behavior, envelopes/sweeps/frame counter/DMA and synchronized browser audio; numerical/capture/device evidence for claimed fidelity. |
|44 debugger | Accurate live source/disassembly/bank/register/memory/PPU/mapper views; supported watch/step/save-state/rewind workflows have bounded side effects/history. |
|45 tutorials | Follow an in-app first-game walkthrough from project creation to compile/play with accessible controls and actionable errors. |
|46 library | Import a reusable font/art/audio asset from the shared library; references and provenance survive save/reload, reuse and deletion/repair. |
|47 offline | Clean offline package runs without internet, rebuilds/reopens projects and preserves user data through upgrades. Historical Tauri proposal requires architecture authorization. |
|48 recovery | All reconstruction/resource/gameplay/save/independent-emulator stages in DESIGN pass for the declared supported games; untouched source-only rebuild equals input. |
|Workspace/code | Interrupted edits recover; undo/reference changes and package/migration round trips preserve data; CLI/browser builds agree; diagnostics map to correct source/assets/banks. |
|Combat/components | Runtime hits/timing/ownership/pool reuse, room/death guards and maximum combined budgets; components/event functions preserve explicit ordering/provenance. |
|Game systems | Complete multi-room dialog/inventory/objective/checkpoint flow with documented reset/restart/save/load ownership; malformed saves fail transactionally. |
|Art/world/audio | Save/reload/import/edit reuse agrees with ROM bytes/pixels/sound; camera seams/collision/uploads and channel arbitration remain within hardware limits. |
|Profiling/replay | Reproduce overload/divergence, identify cause, repair and measure improvement; estimates distinguish actual cycles/scanline/ROM/RAM/OAM results. Missing assertions fail including zero. |
|Release | Clean-source/package deterministic rebuild, playable integrated walkthrough, correct manifests/notices/headers/vectors, supported controller/audio/world/save and independent evidence. |

### Required edge-case profiles

These consolidate the detailed handoff's mandatory cases; use existing suites rather than recreating their proposed filenames.

- Compiler/layout: production API minimal/math/input/text/data/assets and includes; nested/forward cross-bank calls/arguments/returns and interrupt-aware mapper writes; fixed-bank reset after switches; every bank window/reserved region/injection/asset length boundary; deterministic ROM+map; file/bank/macro provenance and instruction-accurate hover/download/breakpoint mappings.
- Language: empty/initialized/assigned/concatenated strings, LEN and heap limits/reuse; BYTE/WORD DATA encoding and RESTORE start/label/include/exhaustion; multi-file persistence/entry selection/name/path/include cycles; carry/borrow/wrap/promotion, mul/div/mod/zero and signed boundaries; SELECT/default/nesting/stack balance; record/array/nested/page offsets and limits; enums/constants/collisions; macro arity/nesting/recursion/label isolation.
- Gameplay: controller fresh/held/released across eight buttons; font/nametable/text/write budgets; signed metasprite offsets/attributes/64OAM/flicker/DMA; animation timing/loop/restart; pool capacity/full sentinel/reuse/index safety; collision containment/touching/zero-size/coordinate edges; wall pixel/tile alignment and middle probes; seeded RNG/ranges; repeated horizontal/vertical seam and attribute/buffer/direction cases.
- Authoring/audio: valid/malformed/cancelled/oversized WAV→DPCM placement/rate/alignment/length; numeric playback and priority/equal/retrigger/resumption/loop/empty IDs; envelope speed/duration/bounds/loop/reference persistence; arpeggio0/4/7/negative/base bounds; SFX JSON export/drop/import/old fields and no partial writes; PNG palette/transparency/tile mapping; metatile/quadrant/individual edit/reference repair; all world map bytes/backtracking/empty/capacity; sprite/timeline names/references/offsets/timing.
- Frontend: fresh bindings/buffer dimensions/lifetimes/memory growth; valid→invalid→valid loads; one loop/bootstrap/input/audio owner; elapsed-time pacing at multiple refresh rates/background/pause; mixed keyboard/gamepad holds/disconnect/visibility/reset; coherent snapshots/BigInt/freeing; stale source/project edits; initial stop-before-execute/resume-once/re-hit/multi-file/bank breakpoints; full RAM/hidden-view budgets; mapped PPU/OAM/viewport/palette geometry.
- Cartridge/bus: normalized iNES/NES2linear/exponent/mapper/submapper/trainer/RAM/NVRAM/timing/console; malformed/overflow/truncated/unsupported cases before allocation; declared backend allocation agreement; NROM mirror/CHR behavior, generatedMMC1 preservation, actual mapper CPU/PPU/IRQ routing and all bus range/side-effect/peek boundaries; transactional sequential load/reset/battery and documented instance limitations.
- CPU: all256 deliberate encodings,151official/56mnemonics and accumulator counted separately; wrapping/indirectJMP/branch/index/page/store/RMW/dummy cycles/accesses; exhaustive binaryADC/SBC with D states, physical/status-stack flags and stack wrap; RESET/NMI/IRQ/BRK/RTI priority/masking/polling/hijack/DMA alignment; required unofficial semantics/bus effects and JAM/reset; immutable fixture hashes/8991records/initial+terminal state; exact pre-instruction comparison and malformed/truncated/mutated-field rejection; native/WASM equivalence and compiler/game regressions.

### Reproduction and review

Use `node scripts/verify.mjs`, `cargo check --workspace --all-targets --locked` and `git diff --check`. Maintained focused suites include `cargo test -p swiss-emulator --test cartridge --locked`, `--test bus_test`, and `--test cpu_trace`; the standalone trace-report CLI remains planned. Pinned fixture notices live in emulator/tests/fixtures/PROVENANCE.md. Real browser/physical-controller/audio and clean-package/host-matrix evidence remain separate from DOM/native/WASM automation.

The integrated demonstration edits/reloads source and art/audio, compiles/downloads ROM+map, plays with keyboard and a physical controller, traverses horizontal/vertical boundaries, collides/spawns/reuses pools, prints text and exercises music/SFX/DPCM. Pause/reset/scale/volume/close/reload and an included-file breakpoint/inspect/continue/re-hit must work. Reuse the authored platform demo and small linked fixtures when one showcase exceeds hardware limits; never weaken placement or assertions to fit.

Reviewer/date/source revision: pending final release review. Test evidence and publishing authorization remain in their respective current documents.

## Development record

This section owns historical observations and the documentation/work audit. [Resume](#current-state) owns current state/results, [DESIGN](#plan) owns the plan, [runtime](#runtime-contracts)/[Studio](#studio-authoring) own contracts and [acceptance](#acceptance-and-release) owns criteria/signoff. Historical findings are not today's task list or commit permission.

### September baseline and repairs

Audit: September12–13,2026 at4610516840d7991bce7e1d879e1f4e2fa831f10e. Windows native tests/format/strict Clippy passed, but minimal production API programs (`SUB Main`, WORD1000+500 and Controller.Read) returned HTTP400/invalid addressing errors. Legacy tests used another generator path. Fresh WASM/browser/canonical trace/device acceptance was not performed then.

| Historical issue | Maintained repair/evidence to reuse |
|---|---|
| Production/legacy helpers, bank shadow/signatures/far calls/reset | Shared generate_banks/compatibility wrapper, reserved RAM, fixed startup/vectors, linked targets and interrupt retry; compile_api/cross_bank/phase_1_10 suites. |
| PC estimates/bank windows/asset overflow/source maps | Checked assembler layout and SourceMapv1/file/bank/snapshots, ROM+map downloads; assembler/linker/asset_validation/ram_overflow/source_map_contract suites. |
| First-map-only output | Checked world directory/banked packing/dedup and World.Load/Tile; world_runtime suite. |
| SFX arpeggio/channel ambiguity | Defaulted arpeggio_sequence, independent timer-pitch/semitones, legacytrack3=DMC; audio runtime/authoring suites. Noise authoring remains an extension gap. |
| Breakpoint first-step/mapper identity, frame pacing, PPU buffers | Stop-before-execute/resume-once, mapper offsets, elapsed-time scheduling and validated PPU/viewport/memory-growth/lifecycle tests. |
| Root-only CI/tracked binaries/reproducibility | Host workspace+pinned WASM jobs, verified fixture/core preparation, ignored generated outputs and portable verifier. |
| Names/trainer/NES2sizes/battery/bus/CPU proof | Project validation, transactional adapter/explicit unsupported geometry, battery partitions/reset, production routing, CPU/interrupt/DMA and immutable8991record native/WASM trace. |

These repairs exist. Pending signoff requires evidence review or demonstrated fixes, not duplicate implementations or a separate validation core. The table preserves the baseline causes and maintained repairs; old raw document copies were removed at the user's request.

### October browser observations

Observed on development based on178f1de, followed by published9243658 and uncommitted increments. These are bounded historical observations, not global fidelity or deterministic-replay guarantees.

| Retained project | Observation |
|---|---|
| platform-demo | Authored text/player/wall/floor and user-reported sound; pause/reset/resume/close and mappedRAM/PPU worked. Eight tiles80–87 atX32/40,Y128/136/144/152 matched art. Full controls visible at1280×720;1x/2x256×240/512×480. |
| studio-acceptance-20261010 | UI-created starter smoke120frames/Title/139280ROMbytes; Village/spawnX48 reloaded, generated source protected/custom main retained, WASM title/player/floor. |
| studio-controls-20261011 | Entry12frames/roundtrip18/wall213-X144 passed; wrongTitle assertion failed againstMeadow, survived reload and passed after repair. |
| studio-entities-20261011 | Lantern/CoinIdle snapping/exact placement reloaded;4instances/20sprites/53RAM. Wrong active2 failed actual3; repaired wall/pickup passed213frames. Door roundtrip107frames/3184014cycles; pickups0→1, active4→3→1→1→4. |
| studio-events-20261011 | Score0, pickup≥1 once-per-visit+100, freshA/Score≥100→Ruins; IDs/rename/reload/49RAM. WrongScore101 failed actual100 at62frames/1843892cycles; repair passed70frames/2082137cycles. Return awarded200 from persistent count, not new collection. Timer preview renderedRuins, then restored rules/Title. |
| studio-combat-20261011 | HeroWalk health2/damage1 at48,128; boxesXoffset4,width8; playerhealth3/immunity30/Bdamage1/reach12/defeatRuins; UP freedB, START/health0 restarted;63RAM. START2/B2/B8/release2/B2 passed26frames/771794cycles after wrong defeat expectation repair; health3, defeats0/0/0/0/1. Reload/WASM actor preview and restoredTitle. |
| studio-spawners-20261011 | SharedHeroWalk redSP0color2=$16/greenSP1=$1A with facing; nests48,64→80/88,64→120, capacity2/delay15/interval10/quota3;125RAM/9slots/32OAM.96frame/2856429cycle roundtrip: counts0/0/1/2/(Title absent)/0/2, active3/3/5/7/0/3/7. Wrong early1 failed actual0 at22frames/652674cycles; repair/reload passed. WASM shared red/green art; CHR/frames/metasprites/main retained. |

Browser repairs: PPU z-order/cache revalidation, Full toolbar height, unblurred capture, stale Inspect/test/project cancellation, read-only protection, placement-linked selection, RGB byte masks and reference-label refresh. Recorded final console had no warnings/errors. Prior sound is not audio-fidelity evidence; physical controls/custom-main/release remain separate gates.

Retained screenshots/logs/projects and cleanup manifest are indexed in `.tools/notes/game-studio-2026-10-10.md`. Checkpoint267native+80frontend+3WASM=350 preceded published9243658/CI38157045058; later combat359 and spawner/reuse380 were local increments. Latest results appear only in Current state. Acceptance server is stopped; inspect processes before restart.

### Reproduced runtime regressions

| Failure | Repair / proof |
|---|---|
| Unaligned boxes skipped middle wall tile (X67vs64) | Leading-edge corners+midpoint; player/patrol production-ROM regression. |
| Eight moving spawns overflowed bank0 | Shared per-emitter worker/guarded room writeback; completed-draw breakpoint verifies36OAM. Mid-frame builder samples are not completed OAM proof. |
| Default/variant draw plus audio/all palettes overlappedD000 | Shared fixed-bank animation frame resolver and combined-feature regression. |
| Duplicate sprite metadata cost ROM | Exact aliases preserve public names/CHR; fixture38→19bytes. Styles add no CHR/frames but use normalCPU/OAM. |
| New left-facing half empty at width1 | Round outward; existing edge regression covers1px/16px both directions. |
| Random power-on ready42 bypassed audio-test startup | Forced collision reproduced the failure; shared helper checks linked STA completion plus RAM marker. Idle NMI/channel assertions retained; wrong saved cooldown expectation and exact expiry boundary also checked. |

### Documentation/work audit and consolidation

October11 instructions: merge old/new plans, avoid duplicate work, review/document existing work, keep fewer docs, no commit. Reviewed19then-existing Markdown inputs and indexed136source/build/example/test inputs, then inventoried174owned files:20docs including the temporary audit,59implementation/build/example,86test/helper,9configuration/patch/notice. The temporary detailed inventory snapshot was removed; compact recovery notes retain audit scope, file changes and findings. This is an ownership/feature/coverage review, not certification of every source line or timing edge.

The temporary landing/audit documents did not reduce duplication enough. Their unique material, contracts, acceptance requirements and historical evidence are now sections of this one guide. All standalone docs were merged and removed. README owns setup/navigation, AGENTS owns operating/recovery rules, and patch/example/fixture READMEs remain beside their inputs. The user requested no raw backup accumulation: all17new raw document copies and the temporary detailed inventory snapshot were removed. No replacement archive was created. Current compact notes document changes; no user projects, credentials or source implementations were removed.

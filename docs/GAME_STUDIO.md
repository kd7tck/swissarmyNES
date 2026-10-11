# Game Studio

The October 11 checkpoint integrates scenes/assets, reusable entities, variables/events and controller tests. [Resume and evidence](IMPLEMENTATION_RESUME.md) · [remaining work](GAME_STUDIO_ROADMAP.md).

## Authoring workflow

1. Run the server and Create playable starter with a unique lowercase project name. It supplies original editable code, art/maps, animations, music/SFX and Title→Meadow↔Ruins. Existing projects are preserved.
2. Select a scene in the graph; assign its populated World cell, spawn, movement and entry audio. Transitions use their displayed order: first matching button/timer wins. Renaming retains IDs; removing a scene repairs start/incoming transitions, while door/event/test references remain for explicit repair.
3. Edit assets in Graphics/Map/World/Sprites/Music/SFX. Reusable entities select an animation/behavior; Place entity adds a scene instance. Select a marker, click to snap at 8 pixels or enter exact X/Y. Set door target/patrol endpoint. Preview uses real CHR/palettes and 16×16 hitbox outlines; P marks player spawn.
4. Add game variables, then scene events with entry/update/button/timer triggers, AND conditions and ordered actions. Move events/actions earlier/later for priority. Variables persist between rooms; arithmetic clamps 0..65535. A scene-change action must be last and stops further rules that update.
5. Save design stores live editor snapshots and metadata. Preview flow compiles its own harness; custom main.swiss stays intact. Starter controls: START enters gameplay, arrows move, B changes rooms, SELECT returns home; contact collects a coin and fresh A inside a doorway enters.
6. Inspect generated code opens virtual read-only game_flow.swiss with source-map provenance. Put custom systems in main.swiss or another include. Standard Compile/Run uses your code; Studio preview/tests use the flow harness.
7. Smoke test executes 120 no-input frames. Controller scenarios save the design, boot a fresh ROM for 10 frames, hold/release buttons for each duration and assert state. Blank expected fields omit assertions;0 asserts zero. The first mismatch stops the run and displays actual state/cycles.

Example rule: Score=0, Meadow/update, pickup count≥1, once per visit, addScore=100. It awards once after any pickup; because the counter persists, returning awards again without another coin. Add Score=0 as a condition for a whole-game one-time reward. Another pressed-A rule with Score≥100 can enter Ruins while preserving score. Custom code accesses gv_Score; rename source symbols in custom code if you rename variables.

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

## Saved schema and limits

Metadata.design is optional; legacy designs default to empty collections. Version 1 uses these fields:

| Object | Fields and validation |
|---|---|
| Design | version=1, title1..120 bytes, core_loop≤4000 bytes, start_scene, optional animation, scenes, optional entities/variables/events/tests |
| Scene | id, name, map_col/map_row, spawn_x/spawn_y, movement, optional music/sfx, transitions, optional instances. Up to255 scenes; IDs unique1..64 bytes; names1..28 ASCII letters/digits/spaces or -_!?. Spawn X0..240/Y0..224. |
| Transition | target scene ID, trigger=a/b/start/select/up/down/left/right/timer, frames. Up to16 per scene, unique triggers; timer1..65535 frames. |
| Entity definition | id/name/animation/behavior;≤32 definitions, unique1..64-byte IDs, name1..120 bytes; behavior=decoration/pickup/door/patrol. Animation identifier must exist with valid nonempty frames/metasprites. |
| Instance | globally unique id, entity ID, x/y, optional target_scene/patrol_end_x;≤16 per scene. Bounds match 16×16 boxes; doors need existing target; patrol X<endpoint≤240. |
| Variable | id/name/initial;≤32, unique1..64-byte IDs; name1..32 letters/digits/underscores starting with a letter, unique ignoring case. initial unsigned 0..65535; generated gv_Name WORD. |
| Event | id/name/scene/trigger/once_per_visit/conditions/actions;≤32, unique1..64-byte IDs, name1..120 bytes, existing scene.≤8 conditions,1..8 actions. |
| Saved test | id/name/steps;≤32 tests, unique1..64-byte IDs, name1..120 bytes,1..64 steps. |
| Step | positive frames, unique controller buttons, optional expected_scene/expected_x/expected_y/expected_pickups/expected_active_entities and expected_variables:{variableID:value}. Total≤600 frames including 10 boot. Timer is not a controller button. |

World cells must be populated; referenced assets must exist. Missing/deleted references are visible and invalid saves are rejected before file writes. Instance/definition selections remain linked; removing a used definition/variable is blocked until its instance/event references are removed. Missing scene/variable test expectations remain coverage and fail rather than disappearing.

### Event forms

- Trigger: `{kind:"enter"}`, `{kind:"update"}`, `{kind:"pressed",button:"a"}`, `{kind:"held",button:"a"}` or `{kind:"timer",frames:120}`. All eight controller buttons are supported.
- Condition: `{source,comparison,value}`. Source is `{kind:"variable",variable:ID}`, or kind pickups/x/y/ticks. Operators: equal/not_equal/less/less_equal/greater/greater_equal, against unsigned 0..65535. No conditions means trigger alone; all conditions are ANDed. Pickup count is 0 without entity slots.
- Actions: kind set/add/subtract with variable/value; scene with scene ID; music with track ID; sfx with effect ID; stop_music. Values are unsigned 0..65535. Add saturates at 65535, subtract saturates at 0. Scene change must be final; audio uses existing arbitration and destination entry audio may replace it.

## Runtime contract

GameFlowStart resets variables/counter and enters the start scene. Entry disables rendering/NMI for map loading, sets spawn/movement/audio, resets timer/player animation and room slots, then restores rendering. GameFlowUpdate reads the controller once, advances a saturating WORD timer, applies transitions, moves the player, processes entities/events, then draws/scrolls. Nonzero map tiles are solid.

Pickups deactivate on contact with the movable player, increment a saturating counter and respawn on every room entry. This is room-local collection, not persistent inventory. Doors require overlap and fresh A; transitions have priority and guards prevent same-update cascades. Patrols move one pixel/update, turning at walls/endpoints; combat is future work. Collision boxes stay 16×16 regardless of art extents. Entry clears shared slots and initializes positions/directions/animations; active entities render even without player animation.

Events run in metadata order and actions sequentially; every matching event can fire until a room change suppresses dispatch. Destination entry rules run on the next eligible update instead of recursively while loading maps. Entry conditions receive one evaluation per visit; false conditions do not retry. Multiple entry rules share the flag. Once-per-visit latches set only after trigger/all conditions match and reset on entry. Timer events compare≥threshold: repeated timers can run every eligible update after it; editor defaults timer triggers to once per visit. Conditions can become true later. An authored entry cycle may change rooms every update, but cannot recursively overflow the stack.

### Resources and source ownership

RAM estimate:9 base bytes +5 optional player animation; with entity slots,3 counter/guard bytes +9×largest-room slots; +2×variables; with events,+1 entry flag +1×events +1 guard if entities did not supply it. Maximum16 slots/player/32 variables/32 events is258 of560 compiler-variable bytes before custom code. Definitions unused by scenes allocate no slots.

OAM estimate sums the largest frame tile count of every placed animation plus player, rejecting>64 sprites. Eight sprites/scanline, CPU/vblank and actual ROM/custom RAM limits remain separate profiling/compiler concerns. Art must suit the preset hitbox.

The virtual flow include is regenerated from validated saved metadata/assets. Writing/deleting it is rejected. Enabling Studio rejects a preexisting physical game_flow.swiss, preserving it for user-directed repair. Immediate input capture makes Save/Preview independent of blur; obsolete async results are discarded. Source maps retain generated-file provenance.

### Test API

POST /api/projects/:name/flow/test accepts frames1..600 (default120) or test_id. The server compiles one owned design/assets snapshot, drains audio between frames and reports passed/message, final/per-step scene/X/Y/pickups/active_entities/entities/variables/generated_ram_bytes, frame count, ROM bytes and CPU cycles. Variables are keyed by stable ID and displayed by current name. Assertion failures return HTTP200/passed:false; malformed requests400.

Each scenario boots fresh for 10 frames. Adjacent identical button holds stay held; empty steps release. Fixed offsets5C0 scene/5C1 ticks/5C3x/5C5y/5C7movable/5C8loaded apply only to isolated MAIN_SOURCE; entity/variable state follows declaration order. Custom code uses names, not these addresses. These tests exclude custom main, physical controls, fidelity and full release acceptance; [evidence](BROWSER_ACCEPTANCE_2026-10-10.md) records actual browser checks.

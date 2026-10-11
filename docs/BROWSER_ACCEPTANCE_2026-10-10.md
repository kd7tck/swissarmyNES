# Browser evidence — October 10–11, 2026

Observed against the development based on main 178f1de, using the production compiler/native/WASM core. October 11 cleanup/local commit is authorized. [Current status](IMPLEMENTATION_RESUME.md) separates these bounded checks from full phase/release and future host-matrix acceptance.

## Authored workflows

| Project | Observation |
|---|---|
| platform-demo | Loaded source/assets; Run rendered text/player/wall/floor. Repeated first Run succeeded. User reported audible output. Pause/reset/resume/Close, nonzero RAM and mapped PPU views worked. OAM tiles80–87 at X32/40, Y128/136/144/152 matched the authored eight tiles. Full toolbar stayed visible at1280×720 (Close Y22–54);1x/2x images256×240/512×480. |
| studio-acceptance-20261010 | UI-created starter; smoke120 frames/Title/139,280 ROM bytes. Renamed Village/spawnX48 survived reload and generated read-only source. Custom main comment survived preview/test. WASM rendered title/player/floor. |
| studio-controls-20261011 | All three scenarios passed: entry12 frames, round trip18, wall213/X144. Wrong expectedTitle failed against actualMeadow; persisted through reload; repair passed. |
| studio-entities-20261011 | Authored Lantern/CoinIdle at200,176; save/reload and snapped120,120/exact restoration worked. Budget4 instances/20 sprites/53 RAM bytes. Production WASM rendered player/coin/door/patrol/decoration. Wrong active count2 failed at3; repair passed213 frames/X144/pickups1/active3. Door round trip passed107 frames/3,184,014cycles; counts0→1 pickups and4→3→1→1→4 active objects. |
| studio-events-20261011 | Authored Score=0 and two Meadow rules: pickup≥1 once per visit adds100; fresh A with Score≥100 enters Ruins. Save/reload preserved IDs/actions/conditions; budget49 RAM bytes. Live Score→Points rename updated assertion labels; restored Score. |

The event scenario has six steps: START2, RIGHT50, release2, A2, B2, release2. ExpectedScore101 deliberately failed at step 2/actual 100,62 total frames/1,843,892cycles. Repair100 passed 70 frames/2,082,137cycles with these states:

| Frame | Scene | Score | Pickups | Active objects |
|---|---|---|---|---|
|12 | Meadow |0 |0 |3 |
|62 | Meadow |100 |1 |2 |
|64 | Meadow |100 |1 |2 |
|66 | Ruins |100 |1 |1 |
|68 | Meadow |200 |1 |3 |
|70 | Meadow |200 |1 |3 |

The return bonus uses the persistent pickup counter; it does not prove new/persistent collection. A temporary Meadow/Score=100/30-frame timer preview changed to RUINS in production WASM without controller input. Preview closed; Title/Score=0/pressed A/repeating restored and saved. Final six-step rerun matched. Generated source was read-only and included gv_Score, guards/latches, comparisons and saturating arithmetic. No warning/error was observed. Cycle totals are measurements, not deterministic replay guarantees.

## Reproduced defects and repairs

- PPU overlay was darkened/obscured at z-index1000; raised above emulator to1100. Stale cached JS survived reload; static no-cache revalidation including304 fixed it.
- Full canvas100vh clipped the toolbar; remaining-height flex sizing fixed it.
- Unblurred edits showed Saved with old metadata; immediate capture/intent flush fixed persistence. Stale Inspect/test work is discarded, read-only Tab is guarded and validation text is retained.
- Entity marker left another definition selected; linked selection fixed it. RGB values clamped to[18,255,255] instead of[18,52,86]; byte masks fixed the pixel buffer and browser colors.
- Variable rename left stale test labels; immediate reference-label refresh fixed it. A separate failing regression protects a variable ID colliding with a scene ID.

## Verification and retained evidence

Final267 native/80 frontend/3 WASM (350), strict fmt/Clippy/syntax/build passed. Earlier matrices253/53/3,260/68/3 and263/74/3 are historical increments. Native tests execute compiled rules/entities; frontend regressions cover lifecycle and authoring; generated WASM matches the canonical CPU trace.

Cleanup retained ignored `.tools/studio-events-pass.jpg`, `studio-events-preview.jpg`, `studio-entities-preview.jpg`, `browser-ppu-20261010.jpg`, `browser-full-20261010.jpg` and events-{native,clippy,build,frontend}-20261011.log. Older screenshots/logs were removed after this summary; exact deletion manifest is in `.tools/notes`. All five local projects remain. Copied acceptance server stopped/removed; restart normally for further work.

Outstanding: physical gamepad/audio fidelity, exported downloads/source-breakpoint walkthroughs, broader art/audio authoring, custom-main execution, clean setup/current remote host matrix and complete [phase signoff](PHASE_40_ACCEPTANCE_CHECKLIST.md). Prior user-confirmed sound is not independent fidelity validation.

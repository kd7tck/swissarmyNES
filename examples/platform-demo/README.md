# Platform demonstration

After completing the repository's normal native/WASM setup, install the editable sample:

```sh
node examples/platform-demo/install.mjs
cargo run
```

Open `http://127.0.0.1:3000`, select project `platform-demo`, and choose Run. The installer creates `projects/platform-demo` with source, included source, assets and project metadata. It preserves an existing project; use `--project ANOTHER_NAME` for a fresh copy.

- Arrow keys move the 16×32 hero. At its initial height, the right wall stops horizontal movement. Move upward above it, then continue right to enter the second room.
- A (keyboard Z) triggers the finite sound effect once per press; its higher priority pauses and then resumes the background tune.
- B (keyboard X) switches rooms immediately and returns the hero to its initial position.
- The hero cycles three frames every five game updates. Its eight OAM tiles use authored CHR patterns, and the title uses an authored ASCII font.

The program also allocates ten entity slots, verifies a full pool, despawns and reuses slot five, and periodically queues a floor tile row with palette attributes. Tile and attribute uploads use consecutive NMIs; all PPU writes finish before the audio update. Bulk room loading and title printing disable rendering and NMI while changing PPU memory.

`assets.mjs` is the compact, authored source for both room maps, CHR, palettes, animation, music and SFX. The installer expands it into editable JSON. Generated project files and ROM/WASM outputs remain ignored.

The production integration test runs installation, project compilation, include/source-map checks, rendered pixels, input movement, the solid-wall stop, the second-room transition, SFX edge triggering, pool capacity/reuse and preservation of user edits:

```sh
cargo test --test platform_demo_execution_test --locked --offline
cargo test --test nmi_workload_execution_test --locked --offline -- --nocapture
```

Native tests and [October browser observations](../../docs/BROWSER_ACCEPTANCE_2026-10-10.md)
cover rendering, controls/debugger presentation and user-confirmed audible output.
Physical controllers and audio fidelity still need separate device evidence. Current
setup/checkpoint commands are in the [root README](../../README.md).

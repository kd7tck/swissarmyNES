use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use swiss_emulator::Emulator;
use swissarmynes::server::{api::compile_source, project};

struct DemoProject {
    name: String,
    path: PathBuf,
}

impl DemoProject {
    fn install() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let name = format!(
            "platform-demo-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("projects")
            .join(&name);
        assert!(
            !path.exists(),
            "Test must never overwrite an existing project"
        );
        let output = Command::new("node")
            .arg("examples/platform-demo/install.mjs")
            .arg("--project")
            .arg(&name)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("Node runtime is required by the project verification matrix");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Self { name, path }
    }
}

impl Drop for DemoProject {
    fn drop(&mut self) {
        // Only the uniquely named project created by this test is removed.
        std::fs::remove_dir_all(&self.path).expect("Test project cleanup failed");
    }
}

fn run(emu: &mut Emulator, frames: usize) {
    for _ in 0..frames {
        emu.step().expect("Integrated demonstration must execute");
    }
}

fn word(emu: &Emulator, address: usize) -> u16 {
    let ram = emu.ram_snapshot();
    u16::from_le_bytes([ram[address], ram[address + 1]])
}

#[test]
fn authored_platform_demo_compiles_includes_renders_moves_collides_and_changes_rooms() {
    let fixture = DemoProject::install();
    let initial = project::get_project(&fixture.name).unwrap();
    let (rom, map) = compile_source(None, Some(fixture.name.clone()), None)
        .expect("Installed demonstration must compile through the project API");
    assert!(
        map.sources.contains_key("rooms.swiss"),
        "Included source must retain debugger provenance"
    );
    let mut emu = Emulator::new();
    emu.load_rom(&rom).unwrap();
    run(&mut emu, 10);
    assert_eq!(word(&emu, 0x05c0), 32);
    assert_eq!(word(&emu, 0x05c2), 128);
    assert_eq!(emu.ram_snapshot()[0x05c5], 255, "First room must load");
    assert_eq!(&emu.ram_snapshot()[0x05f7..0x05f9], &[255, 255]);
    assert_eq!(word(&emu, 0x05f9), 5);
    // A copied read-only frame is safe while the paused emulator owns the storage.
    let pixels =
        unsafe { std::slice::from_raw_parts(emu.get_pixels(), emu.get_pixels_len()) }.to_vec();
    let colors: std::collections::BTreeSet<_> = pixels
        .chunks_exact(4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    assert!(
        colors.len() >= 3,
        "Authored background, font and animated hero must render distinct colors"
    );
    assert_eq!(
        emu.ram_snapshot()[0x001b],
        8,
        "Hero must produce eight shadow OAM entries"
    );
    emu.set_button(0, 0, true); // A triggers a finite SFX once per pressed edge.
    run(&mut emu, 2);
    assert_eq!(emu.ram_snapshot()[0x05c6], 1);
    run(&mut emu, 4);
    assert_eq!(emu.ram_snapshot()[0x05c6], 1, "Held A must not retrigger");
    emu.set_button(0, 0, false);
    emu.set_button(0, 7, true); // Right walks into the authored column-22 wall.
    run(&mut emu, 200);
    assert_eq!(
        word(&emu, 0x05c0),
        160,
        "A 16-pixel-wide hero stops before x176 wall"
    );
    assert_eq!(emu.ram_snapshot()[0x05c9], 1);
    emu.set_button(0, 7, false);
    emu.set_button(0, 4, true); // Up moves above the wall before crossing the room seam.
    run(&mut emu, 40);
    emu.set_button(0, 4, false);
    assert!(word(&emu, 0x05c2) < 97);
    emu.set_button(0, 7, true);
    run(&mut emu, 100);
    emu.set_button(0, 7, false);
    assert_eq!(
        emu.ram_snapshot()[0x05c4],
        1,
        "Rightward seam must select room1"
    );
    assert_eq!(emu.ram_snapshot()[0x05c5], 255);
    assert!(word(&emu, 0x05c0) < 40);
    // Reinstallation must preserve a user's edited project rather than overwrite it.
    project::save_project(
        &fixture.name,
        Some("' User edit\nSUB Main()\nEND SUB"),
        None,
    )
    .unwrap();
    let output = Command::new("node")
        .arg("examples/platform-demo/install.mjs")
        .arg("--project")
        .arg(&fixture.name)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(output.status.success());
    let retained = project::get_project(&fixture.name).unwrap();
    assert!(retained.source.starts_with("' User edit"));
    assert_eq!(
        retained.assets.unwrap().chr_bank,
        initial.assets.unwrap().chr_bank
    );
}

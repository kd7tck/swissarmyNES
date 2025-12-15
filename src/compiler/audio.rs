pub const PERIOD_TABLE_ADDR: u16 = 0xD000;
pub const MUSIC_DATA_ADDR: u16 = 0xD100;

/// NTSC Period Table for octaves 0-7 (C to B)
/// Indices: 0-11 = Octave 0, 12-23 = Octave 1, etc.
/// 12 notes per octave * 8 octaves = 96 entries (192 bytes for 16-bit periods).
/// We need to inject this as a binary blob.
/// 6502 will index it by Note Index * 2.
pub fn generate_period_table() -> Vec<u8> {
    // NTSC Periods for C, C#, D, D#, E, F, F#, G, G#, A, A#, B
    // Octave 0 is very low.
    // Source: http://wiki.nesdev.com/w/index.php/APU_period_table
    // We will use a standard lookup table.
    // Let's use octaves 1-8 for useful range, or 0-7 if that's standard.
    // Note 0 = C-1? No, let's say Note 0 = C-2.
    // Let's copy a standard table.

    // NTSC Periods (Low Byte, High Byte)
    // C-0 to B-7
    let periods: [u16; 96] = [
        // Octave 0
        0x06AE, 0x064E, 0x05F4, 0x059E, 0x054D, 0x0501, 0x04B8, 0x0474, 0x0432, 0x03F4, 0x03B8,
        0x0380, // Octave 1
        0x0357, 0x0327, 0x02FA, 0x02CF, 0x02A6, 0x0280, 0x025C, 0x023A, 0x0219, 0x01FA, 0x01DC,
        0x01C0, // Octave 2
        0x01AB, 0x0193, 0x017D, 0x0167, 0x0153, 0x0140, 0x012E, 0x011D, 0x010C, 0x00FD, 0x00EE,
        0x00E0, // Octave 3
        0x00D5, 0x00C9, 0x00BE, 0x00B3, 0x00A9, 0x00A0, 0x0097, 0x008E, 0x0086, 0x007E, 0x0077,
        0x0070, // Octave 4
        0x006A, 0x0064, 0x005F, 0x0059, 0x0054, 0x0050, 0x004B, 0x0047, 0x0043, 0x003F, 0x003B,
        0x0038, // Octave 5
        0x0035, 0x0032, 0x002F, 0x002C, 0x002A, 0x0028, 0x0025, 0x0023, 0x0021, 0x001F, 0x001D,
        0x001C, // Octave 6
        0x001A, 0x0019, 0x0017, 0x0016, 0x0015, 0x0014, 0x0012, 0x0011, 0x0010, 0x000F, 0x000E,
        0x000E, // Octave 7
        0x000D, 0x000C, 0x000B, 0x000B, 0x000A, 0x000A, 0x0009, 0x0008, 0x0008, 0x0007, 0x0007,
        0x0007,
    ];

    let mut blob = Vec::with_capacity(periods.len() * 2);
    for p in periods {
        blob.push((p & 0xFF) as u8); // Low
        blob.push(((p >> 8) & 0xFF) as u8); // High
    }
    blob
}

use crate::server::project::ProjectAssets;

/// Compiles audio tracks into a binary format.
/// Format for each track:
/// [Length (1 byte)], [Note Data...]
/// Note Data: [Duration (1 byte)], [Pitch Index (1 byte)]
/// Pitch Index: 0-95 (Valid), 255 (Rest)
/// End of Track: implicit by Length? Or 0x00 duration?
/// Let's use a pointer table at the start of MUSIC_DATA_ADDR?
/// Simplification: Just one track for now? Or concatenated?
/// User memory says "3-channel (Pulse 1, Pulse 2, Triangle)".
///
/// Current Sound Engine in Codegen is very simple (PlaySfx).
/// This refactor aims to support "Music" eventually, but for now we need to inject the data tables.
///
/// Let's just return an empty music blob if no tracks, or a simple test pattern.
/// The ProjectAssets has `audio_tracks`.
pub fn compile_audio_data(assets: &Option<ProjectAssets>) -> Vec<u8> {
    // Header: 3 Pointers to Track Data (Pulse 1, Pulse 2, Triangle)
    // Relative to MUSIC_DATA_ADDR? Or Absolute?
    // Let's just output raw data and let the engine handle it later.
    // For this task, we just need to satisfy the "Inject Binary Data" requirement.
    // The user wants to replace .BYTE directives.

    // If we have actual tracks, we can compile them.
    // For now, let's generate a placeholder or compile what's there.

    let mut blob = Vec::new();

    if let Some(assets) = assets {
        for track in &assets.audio_tracks {
            // Compile track
            // Format: [Duration, Pitch, Duration, Pitch, ..., 0 (End)]
            for note in &track.notes {
                blob.push(note.duration);
                blob.push(note.pitch);
            }
            blob.push(0); // Terminator
        }
    }

    // Pad if empty to avoid empty blob issues?
    if blob.is_empty() {
        blob.push(0);
    }

    blob
}

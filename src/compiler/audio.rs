pub const PERIOD_TABLE_ADDR: u16 = 0xD000;
pub const MUSIC_DATA_ADDR: u16 = 0xD100;

/// NTSC Period Table for octaves 0-7 (C to B)
/// Indices: 0-11 = Octave 0, 12-23 = Octave 1, etc.
/// 12 notes per octave * 8 octaves = 96 entries (192 bytes for 16-bit periods).
pub fn generate_period_table() -> Vec<u8> {
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

/// Compiles audio tracks into a binary format injected at MUSIC_DATA_ADDR ($D100).
/// Structure:
/// Header:
///   Count (1 byte): Number of tracks
///   Pointers (2 * Count bytes): Absolute addresses of each track data
/// Data:
///   [Track 0 Data]
///   [Track 1 Data]
///   ...
///
/// Track Data Format:
///   Channel (1 byte): 0=Pulse1, 1=Pulse2, 2=Triangle, 3=Noise (Only 0-2 supported by engine currently)
///   Sequence of Notes:
///     [Duration, Pitch]
///     ...
///     [0] (Terminator)
pub fn compile_audio_data(assets: &Option<ProjectAssets>) -> Vec<u8> {
    let mut blob = Vec::new();

    if let Some(assets) = assets {
        let count = assets.audio_tracks.len();
        if count > 127 {
            // Limit to fit index arithmetic or reasonable bounds
            // For now, let's assume < 128 tracks
        }
        blob.push(count as u8);

        // Reserve space for pointers
        let pointer_table_size = count * 2;
        let data_start_offset = 1 + pointer_table_size;

        // Pointers will be filled later, we insert placeholders
        for _ in 0..pointer_table_size {
            blob.push(0);
        }

        let mut current_offset = data_start_offset;

        for (i, track) in assets.audio_tracks.iter().enumerate() {
            // Calculate absolute address of this track
            // Base Address + current_offset
            let abs_addr = MUSIC_DATA_ADDR as usize + current_offset;

            // Update pointer in table
            let ptr_idx = 1 + (i * 2);
            blob[ptr_idx] = (abs_addr & 0xFF) as u8;
            blob[ptr_idx + 1] = ((abs_addr >> 8) & 0xFF) as u8;

            // Write Track Data
            // 1. Channel
            blob.push(track.envelope);
            current_offset += 1;

            // 2. Notes
            for note in &track.notes {
                blob.push(note.duration);
                blob.push(note.pitch);
                current_offset += 2;
            }

            // 3. Terminator
            blob.push(0);
            current_offset += 1;
        }
    } else {
        blob.push(0); // Count = 0
    }

    // Pad to ensure we don't return an empty vector if that's an issue
    if blob.is_empty() {
        blob.push(0);
    }

    blob
}

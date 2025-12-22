pub const PERIOD_TABLE_ADDR: u16 = 0xD000;
pub const MUSIC_DATA_ADDR: u16 = 0xD100;
pub const MUSIC_DATA_SIZE: usize = 0x380; // 896 bytes ($D100-$D480)

pub const SAMPLE_TABLE_ADDR: u16 = 0xD480;
pub const SAMPLE_TABLE_SIZE: usize = 0x80; // 128 bytes ($D480-$D500)

pub const SFX_TABLE_ADDR: u16 = 0xD900;
pub const SFX_TABLE_SIZE: usize = 0x100; // 256 bytes ($D900-$DA00)

pub const ENVELOPE_TABLE_ADDR: u16 = 0xDA00;
pub const ENVELOPE_TABLE_SIZE: usize = 0x600; // 1536 bytes ($DA00-$E000)

pub const SAMPLE_DATA_ADDR: u16 = 0xE040;
pub const SAMPLE_DATA_SIZE: usize = 0x1EC0; // 7872 bytes ($E040-$FF00)

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
use std::iter;

/// Compiles DPCM samples into two blobs:
/// 1. `samples_blob`: The raw sample data, padded and aligned for DMC.
/// 2. `table_blob`: A lookup table (Address Byte, Length Byte) for each sample.
pub fn compile_samples(assets: &Option<ProjectAssets>) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut samples_blob = Vec::new();
    let mut table_blob = Vec::new();

    let start_addr = SAMPLE_DATA_ADDR as usize;
    let mut current_addr = start_addr;

    if let Some(assets) = assets {
        for sample in &assets.samples {
            // 1. Alignment check
            let alignment_needed = current_addr % 64;
            if alignment_needed != 0 {
                let padding = 64 - alignment_needed;
                samples_blob.extend(iter::repeat_n(0, padding));
                current_addr += padding;
            }

            // 2. Prepare Data
            let mut data = sample.data.clone();
            if data.is_empty() {
                data.push(0);
            }
            if data.len() > 4081 {
                data.truncate(4081);
            }

            // Length must be 16*N + 1
            // We can satisfy this by extending to next multiple of 16, plus 1.
            // (len - 1) % 16 == 0
            let remainder = (data.len().saturating_sub(1)) % 16;
            if remainder != 0 {
                let needed = 16 - remainder;
                data.extend(iter::repeat_n(0, needed));
            }

            // Recalculate L
            let l_val = ((data.len() - 1) / 16) as u8;

            // Calculate A
            // A = (Address - $C000) >> 6
            // Address must be in range $C000-$FFFF.
            let a_val = if current_addr >= 0xC000 {
                (((current_addr as u32) - 0xC000) >> 6) as u8
            } else {
                0
            };

            // Store in table
            table_blob.push(a_val);
            table_blob.push(l_val);

            // Append data
            let len = data.len();
            samples_blob.extend(data);
            current_addr += len;
        }
    }

    // Pad table to 128 bytes (64 entries)
    if table_blob.len() < 128 {
        let needed = 128 - table_blob.len();
        table_blob.extend(iter::repeat_n(0, needed));
    }

    // Check Limits
    if table_blob.len() > SAMPLE_TABLE_SIZE {
        return Err(format!(
            "Sample Table exceeds limit of {} bytes (got {})",
            SAMPLE_TABLE_SIZE,
            table_blob.len()
        ));
    }
    if samples_blob.len() > SAMPLE_DATA_SIZE {
        return Err(format!(
            "DPCM Samples exceed limit of {} bytes (got {})",
            SAMPLE_DATA_SIZE,
            samples_blob.len()
        ));
    }

    Ok((samples_blob, table_blob))
}

pub fn compile_envelopes(assets: &Option<ProjectAssets>) -> Result<Vec<u8>, String> {
    let mut blob = Vec::new();
    if let Some(assets) = assets {
        let user_env_count = assets.envelopes.len();
        let sfx_count = assets.sound_effects.len();
        let total_count = user_env_count + (sfx_count * 3); // Vol, Pitch, Duty per SFX

        blob.push(total_count as u8);

        // Pointers
        let pointer_table_size = total_count * 2;
        blob.extend(iter::repeat_n(0, pointer_table_size));

        let start_addr = ENVELOPE_TABLE_ADDR as usize;
        let mut current_offset = 1 + pointer_table_size;

        let mut env_idx = 0;

        // 1. User Envelopes
        for env in &assets.envelopes {
            let abs_addr = start_addr + current_offset;
            let ptr_idx = 1 + (env_idx * 2);
            blob[ptr_idx] = (abs_addr & 0xFF) as u8;
            blob[ptr_idx + 1] = ((abs_addr >> 8) & 0xFF) as u8;
            env_idx += 1;

            // Data
            // Loop Index
            blob.push(env.loop_index.unwrap_or(0xFF));
            current_offset += 1;

            for (val, dur) in &env.steps {
                blob.push(*val as u8); // Cast i8 to u8
                blob.push(*dur);
                current_offset += 2;
            }

            // Terminator
            blob.push(0); // Val
            blob.push(0); // Dur = 0
            current_offset += 2;
        }

        // 2. SFX Envelopes
        for sfx in &assets.sound_effects {
            // Speed (Duration)
            let speed = if sfx.speed == 0 { 1 } else { sfx.speed };
            let loop_val = if sfx.does_loop { 0 } else { 0xFF };

            // Vol Env
            {
                let abs_addr = start_addr + current_offset;
                let ptr_idx = 1 + (env_idx * 2);
                blob[ptr_idx] = (abs_addr & 0xFF) as u8;
                blob[ptr_idx + 1] = ((abs_addr >> 8) & 0xFF) as u8;
                env_idx += 1;

                blob.push(loop_val);
                current_offset += 1;

                for &val in &sfx.vol_sequence {
                    blob.push(val);
                    blob.push(speed);
                    current_offset += 2;
                }
                blob.push(0);
                blob.push(0);
                current_offset += 2;
            }

            // Pitch Env
            {
                let abs_addr = start_addr + current_offset;
                let ptr_idx = 1 + (env_idx * 2);
                blob[ptr_idx] = (abs_addr & 0xFF) as u8;
                blob[ptr_idx + 1] = ((abs_addr >> 8) & 0xFF) as u8;
                env_idx += 1;

                blob.push(loop_val);
                current_offset += 1;

                for &val in &sfx.pitch_sequence {
                    blob.push(val as u8); // Cast i8 to u8
                    blob.push(speed);
                    current_offset += 2;
                }
                blob.push(0);
                blob.push(0);
                current_offset += 2;
            }

            // Duty Env
            {
                let abs_addr = start_addr + current_offset;
                let ptr_idx = 1 + (env_idx * 2);
                blob[ptr_idx] = (abs_addr & 0xFF) as u8;
                blob[ptr_idx + 1] = ((abs_addr >> 8) & 0xFF) as u8;
                env_idx += 1;

                blob.push(loop_val);
                current_offset += 1;

                for &val in &sfx.duty_sequence {
                    blob.push(val);
                    blob.push(speed);
                    current_offset += 2;
                }
                blob.push(0);
                blob.push(0);
                current_offset += 2;
            }
        }
    } else {
        blob.push(0);
    }
    if blob.is_empty() {
        blob.push(0);
    }
    if blob.len() > ENVELOPE_TABLE_SIZE {
        return Err(format!(
            "Envelope Data exceeds limit of {} bytes (got {})",
            ENVELOPE_TABLE_SIZE,
            blob.len()
        ));
    }
    Ok(blob)
}

pub fn compile_sfx_data(assets: &Option<ProjectAssets>) -> Result<Vec<u8>, String> {
    let mut blob = Vec::new();
    if let Some(assets) = assets {
        let count = assets.sound_effects.len();
        blob.push(count as u8);

        // Pointers
        let pointer_table_size = count * 2;
        let data_start_offset = 1 + pointer_table_size;
        blob.extend(iter::repeat_n(0, pointer_table_size));

        let mut current_offset = data_start_offset;

        let user_env_count = assets.envelopes.len();

        for (i, sfx) in assets.sound_effects.iter().enumerate() {
            let abs_addr = SFX_TABLE_ADDR as usize + current_offset;
            let ptr_idx = 1 + (i * 2);
            blob[ptr_idx] = (abs_addr & 0xFF) as u8;
            blob[ptr_idx + 1] = ((abs_addr >> 8) & 0xFF) as u8;

            // SFX Data Structure:
            // Channel (1)
            // Priority (1)
            // VolEnvID (1)
            // PitchEnvID (1)
            // DutyEnvID (1)

            blob.push(sfx.channel);
            blob.push(sfx.priority);

            // Calculate Envelope IDs
            let base_env_id = user_env_count + (i * 3);
            blob.push(base_env_id as u8); // Vol
            blob.push((base_env_id + 1) as u8); // Pitch
            blob.push((base_env_id + 2) as u8); // Duty

            current_offset += 5;
        }
    } else {
        blob.push(0);
    }
    if blob.is_empty() {
        blob.push(0);
    }
    if blob.len() > SFX_TABLE_SIZE {
        return Err(format!(
            "SFX Table exceeds limit of {} bytes (got {})",
            SFX_TABLE_SIZE,
            blob.len()
        ));
    }
    Ok(blob)
}

/// Compiles audio tracks into a binary format injected at MUSIC_DATA_ADDR ($D100).
///
/// # Binary Format Specification
///
/// ## Header
/// The audio data block starts with a header table that allows the engine to locate tracks by ID.
/// - **Count** (1 byte): The total number of audio tracks available.
/// - **Pointers** (2 * Count bytes): A table of 16-bit absolute addresses (Little Endian) pointing to the start of each track's data.
///
/// ## Track Data
/// Each track is a self-contained sequence of bytes:
/// - **Channel** (1 byte): The hardware channel index to use.
///   - `0`: Pulse 1
///   - `1`: Pulse 2
///   - `2`: Triangle
///   - `3`: DMC (Uses Sample Table)
/// - **Instrument** (1 byte): The hardware envelope or duty cycle setting.
///   - For Pulse: Bits 7-6 = Duty, Bits 3-0 = Volume/Envelope.
///   - For Triangle: Linear Counter Load.
///   - For DMC: Rate Index (0-15).
/// - **Priority** (1 byte): The priority level of the track. Higher values interrupt lower values.
/// - **VolEnv** (1 byte): Index of Volume Envelope ($FF = None).
/// - **PitchEnv** (1 byte): Index of Pitch Envelope ($FF = None).
/// - **Note Sequence**: A stream of `[Duration, Pitch]` pairs.
///   - **Duration** (1 byte): Frames to play. 0 = End.
///   - **Pitch** (1 byte): Period Table Index. For DMC: Sample Index.
///     - Index `$FF` (255) represents **Silence**.
pub fn compile_audio_data(assets: &Option<ProjectAssets>) -> Result<Vec<u8>, String> {
    let mut blob = Vec::new();

    if let Some(assets) = assets {
        let count = assets.audio_tracks.len();
        blob.push(count as u8);

        // Reserve space for pointers
        let pointer_table_size = count * 2;
        let data_start_offset = 1 + pointer_table_size;

        // Pointers will be filled later, we insert placeholders
        blob.extend(iter::repeat_n(0, pointer_table_size));

        let mut current_offset = data_start_offset;

        for (i, track) in assets.audio_tracks.iter().enumerate() {
            // Calculate absolute address of this track
            // Base Address + current_offset
            let abs_addr = MUSIC_DATA_ADDR as usize + current_offset;

            // Update pointer in table
            let ptr_idx = 1 + (i * 2);
            blob[ptr_idx] = (abs_addr & 0xFF) as u8;
            blob[ptr_idx + 1] = ((abs_addr >> 8) & 0xFF) as u8;

            // Write Track Header
            // 1. Channel
            blob.push(track.channel);
            // 2. Instrument
            blob.push(track.instrument);
            // 3. Priority
            blob.push(track.priority);
            // 4. VolEnv
            blob.push(track.vol_env.unwrap_or(0xFF));
            // 5. PitchEnv
            blob.push(track.pitch_env.unwrap_or(0xFF));
            // 6. ArpeggioEnv
            blob.push(track.arpeggio_env.unwrap_or(0xFF));

            current_offset += 6;

            // 3. Notes
            // We need to sort notes by `col` and insert silence/rests for gaps.
            // Pitch 255 ($FF) is reserved for Silence.

            // Clone and sort notes by column
            let mut notes = track.notes.clone();
            notes.sort_by_key(|n| n.col);

            let mut current_time: u16 = 0;

            for note in notes {
                if note.duration == 0 {
                    continue;
                }

                // Note col is u8, so it won't exceed 255, but current_time might accumulate if we were tracking absolute time.
                // In the tracker UI, col is the grid column index (step).
                // We assume each step is 8 frames (approx 130ms), matching the frontend default.
                const FRAMES_PER_STEP: u16 = 8;
                let note_start_time = (note.col as u16) * FRAMES_PER_STEP;

                // Check for gap
                if note_start_time > current_time {
                    let gap_duration = (note_start_time - current_time) as u8;
                    // Insert Silence
                    blob.push(gap_duration);
                    blob.push(0xFF); // Silence
                    current_offset += 2;
                }

                // If note starts before current_time, it's an overlap.
                if note_start_time >= current_time {
                    blob.push(note.duration);
                    blob.push(note.pitch);
                    current_offset += 2;
                    current_time = note_start_time + (note.duration as u16);
                }
            }

            // 4. Terminator
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

    if blob.len() > MUSIC_DATA_SIZE {
        return Err(format!(
            "Music Data exceeds limit of {} bytes (got {})",
            MUSIC_DATA_SIZE,
            blob.len()
        ));
    }

    Ok(blob)
}

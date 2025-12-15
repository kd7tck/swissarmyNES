use crate::server::project::AudioTrack;

/// Compiles audio tracks into a byte stream for the NES sound engine.
///
/// Format:
/// Header:
///   - Byte 0: Number of tracks (should be 3)
///   - Byte 1-2: Pointer to Track 1 Data
///   - Byte 3-4: Pointer to Track 2 Data
///   - Byte 5-6: Pointer to Track 3 Data
///
/// Track Data Stream:
///   - A sequence of (Note, Duration) pairs.
///   - Note:
///     - 0: Rest (Silence)
///     - 1..N: Note Index (Lookup table for Periods)
///     - 0xFF: End of Stream (Loop)
///   - Duration:
///     - Number of frames (or ticks) to play the note.
///
/// Tracker Data:
///   - 32 columns (steps).
///   - We iterate 0..31.
///   - If a note exists at col `i`, we emit it.
///   - If no note, we emit a Rest.
///   - Duration is determined by the distance to the next note or end of step.
///   - Simplification: Each column is a fixed duration (e.g. 8 frames = 1/8th second approx).
///     So stream is just [Note, Note, Note...] where Note 0 is rest.
///     This eliminates the "Duration" byte, making the engine simpler.
///     Let's try Fixed Step first. 32 steps. 1 byte per step.
///     32 bytes per track.
///     End with 0xFF (Loop).
///     Total: 33 bytes per track.
///
/// Note Mapping:
///   - The tracker `row` 0 is the highest pitch.
///   - We need a Period Table in the engine.
///   - `row` maps to an index in the Period Table.
///   - Note value = `row + 1`. 0 = Rest.
///
pub fn compile_audio(tracks: &[AudioTrack]) -> Vec<u8> {
    let mut buffer = Vec::new();
    let num_tracks = 3;

    // We reserve space for the header (1 + 2*3 = 7 bytes)
    buffer.push(num_tracks as u8); // Num Tracks

    let mut track_offsets = vec![0u16; num_tracks];
    // Placeholder for offsets
    for _ in 0..num_tracks {
        buffer.push(0); // Low
        buffer.push(0); // High
    }

    // Generate Data for each track
    // If tracks provided < 3, we generate empty tracks for the remainder.
    for i in 0..num_tracks {
        // Record start offset
        let start_offset = buffer.len() as u16;
        track_offsets[i] = start_offset;

        if i < tracks.len() {
            let track = &tracks[i];
            // Convert Grid to Stream
            // 32 Steps.
            let mut steps = vec![0u8; 32]; // 0 = Rest

            for note in &track.notes {
                if (note.col as usize) < 32 {
                    // Map row to Note Index.
                    // Row 0 = Highest Note.
                    // We support 24 rows (2 octaves).
                    // Note Index = Row + 1. (1 based).
                    // Let's map Row 0 to Index 24, Row 23 to Index 1.
                    // 0 is Rest.
                    let note_val = if note.row < 24 {
                        24 - note.row
                    } else {
                        1 // Lowest valid
                    };
                    steps[note.col as usize] = note_val;
                }
            }

            // Write steps
            for step in steps {
                buffer.push(step);
            }
        } else {
            // Write empty track (just loop terminator, or silent steps?)
            // If we write just 0xFF, it loops immediately.
            // But if other tracks are playing, this one should just be silent for 32 steps?
            // The engine advances Step counter 0-31 globally.
            // If we have less data, we might read garbage.
            // So we MUST pad to 32 steps of silence.
            buffer.extend(std::iter::repeat_n(0, 32));
        }

        // Write Loop terminator
        buffer.push(0xFF);
    }

    // Backpatch offsets
    for (i, offset) in track_offsets.iter().enumerate() {
        let pos = 1 + i * 2;
        buffer[pos] = (offset & 0xFF) as u8;
        buffer[pos + 1] = (offset >> 8) as u8;
    }

    buffer
}

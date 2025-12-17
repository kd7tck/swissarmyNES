class AudioTracker {
    constructor(rootId) {
        this.root = document.getElementById(rootId);
        this.currentTrackIndex = 0;
        this.currentInstrument = 0x9F; // Default instrument (Max Vol, 50% Duty)
        this.tracks = [
            { name: "Track 1", notes: [], channel: 0, instrument: 0x9F },
            { name: "Track 2", notes: [], channel: 1, instrument: 0x9F },
            { name: "Track 3", notes: [], channel: 2, instrument: 0xFF } // Triangle Linear On
        ];

        // Configuration
        this.rows = 24; // 2 octaves
        this.cols = 32; // 32 steps
        this.noteNames = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        this.defaultDuration = 8; // Default duration in frames (approx 130ms)

        this.initUI();
    }

    initUI() {
        this.renderGrid();
        this.bindEvents();
    }

    bindEvents() {
        const trackSelect = document.getElementById('audio-track-select');
        const instrumentSelect = document.getElementById('audio-instrument-select');

        if (trackSelect) {
            trackSelect.addEventListener('change', (e) => {
                this.currentTrackIndex = parseInt(e.target.value);
                this.loadTrackUI();
            });
        }

        if (instrumentSelect) {
            instrumentSelect.addEventListener('change', (e) => {
                // Parse hex or int from value
                this.currentInstrument = parseInt(e.target.value);
                this.tracks[this.currentTrackIndex].instrument = this.currentInstrument;
            });
        }

        const btnPlay = document.getElementById('btn-play-track');
        if (btnPlay) {
            btnPlay.addEventListener('click', () => {
                // Placeholder for local preview if we add it
                console.log("Preview not implemented in JS yet. Compile to hear.");
            });
        }
    }

    renderGrid() {
        this.root.innerHTML = ''; // Clear existing

        // Add row labels and cells
        // Start from high pitch (top) to low pitch (bottom)
        // e.g. Octave 4 down to Octave 3

        let startOctave = 4;
        let noteIndex = this.noteNames.length - 1; // Start at B

        for (let r = 0; r < this.rows; r++) {
            // Calculate note name
            const currentNote = this.noteNames[noteIndex];
            const isBlackKey = currentNote.includes('#');
            const labelText = `${currentNote}${startOctave}`;

            // Label
            const label = document.createElement('div');
            label.className = `tracker-row-label ${isBlackKey ? 'black-key' : ''}`;
            label.textContent = labelText;
            this.root.appendChild(label);

            // Cells
            for (let c = 0; c < this.cols; c++) {
                const cell = document.createElement('div');
                cell.className = 'tracker-cell';
                cell.dataset.row = r; // Row 0 is highest pitch
                cell.dataset.col = c; // Time step
                cell.dataset.pitch = (startOctave * 12) + noteIndex; // Absolute pitch if needed

                cell.addEventListener('mousedown', (e) => this.toggleNote(r, c, cell));

                this.root.appendChild(cell);
            }

            // Decrement note
            noteIndex--;
            if (noteIndex < 0) {
                noteIndex = this.noteNames.length - 1;
                startOctave--;
            }
        }
    }

    toggleNote(row, col, cellElement) {
        const track = this.tracks[this.currentTrackIndex];

        // Check if note exists
        const existingIndex = track.notes.findIndex(n => n.row === row && n.col === col);

        if (existingIndex >= 0) {
            // Remove note
            track.notes.splice(existingIndex, 1);
            cellElement.classList.remove('active');
        } else {
            // Add note
            // For monophonic tracks (NES channels), we might want to clear other notes in the same column?
            // For now, let's allow overlapping in UI but backend can resolve.
            // Or simpler: Enforce monophony per column?
            // Let's keep it simple: Add note.
            track.notes.push({
                row: row,
                col: col,
                pitch: cellElement.dataset.pitch, // Stored as string or int
                duration: this.defaultDuration
            });
            cellElement.classList.add('active');
        }
    }

    loadTrackUI() {
        // Clear all active cells
        const cells = this.root.querySelectorAll('.tracker-cell');
        cells.forEach(c => c.classList.remove('active'));

        // Update Instrument Select
        const instrumentSelect = document.getElementById('audio-instrument-select');
        if (instrumentSelect) {
            const trk = this.tracks[this.currentTrackIndex];
            // If instrument not set, default it
            if (trk.instrument === undefined) trk.instrument = 0x9F;
            instrumentSelect.value = trk.instrument;

            // Disable instrument select for Triangle? Or allow Linear Counter control?
            // Channel is fixed per track index usually?
            // 0: Pulse 1, 1: Pulse 2, 2: Triangle
            // If Triangle, options might be limited.
        }

        // Apply notes from current track
        const track = this.tracks[this.currentTrackIndex];
        track.notes.forEach(note => {
            // Find cell
            const cell = this.root.querySelector(`.tracker-cell[data-row="${note.row}"][data-col="${note.col}"]`);
            if (cell) {
                cell.classList.add('active');
            }
        });
    }

    getData() {
        return {
            tracks: this.tracks
        };
    }

    loadData(data) {
        if (data && data.audio_tracks) {
            // Map loaded tracks to internal structure
            // Ensure we have 3 tracks
            this.tracks = data.audio_tracks.map((t, i) => ({
                name: t.name || `Track ${i+1}`,
                notes: t.notes.map(n => ({
                    row: n.row,
                    col: n.col || n.step, // Handle legacy or different naming if needed
                    pitch: n.pitch,
                    duration: n.duration
                })),
                channel: (t.channel !== undefined) ? t.channel : i, // Default to index if missing
                instrument: (t.instrument !== undefined) ? t.instrument : ((i===2)?0xFF:0x9F) // Default inst
            }));

            // Fill up to 3 if missing
            while (this.tracks.length < 3) {
                let i = this.tracks.length;
                this.tracks.push({
                    name: `Track ${i+1}`,
                    notes: [],
                    channel: i,
                    instrument: (i===2)?0xFF:0x9F
                });
            }
        } else {
            // Reset
             this.tracks = [
                { name: "Track 1", notes: [], channel: 0, instrument: 0x9F },
                { name: "Track 2", notes: [], channel: 1, instrument: 0x9F },
                { name: "Track 3", notes: [], channel: 2, instrument: 0xFF }
            ];
        }
        this.loadTrackUI();
    }
}

// Export for global usage
window.AudioTracker = AudioTracker;

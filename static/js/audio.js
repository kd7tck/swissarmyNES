class AudioTracker {
    constructor(rootId) {
        this.root = document.getElementById(rootId);
        this.currentTrackIndex = 0;
        this.currentInstrument = 0x9F; // Default instrument
        this.tracks = [
            { name: "Track 1", notes: [], channel: 0, instrument: 0x9F, priority: 0 },
            { name: "Track 2", notes: [], channel: 1, instrument: 0x9F, priority: 0 },
            { name: "Track 3", notes: [], channel: 2, instrument: 0xFF, priority: 0 },
            { name: "Track 4", notes: [], channel: 3, instrument: 0x0F, priority: 0 } // DMC
        ];
        this.samples = [];

        // Configuration
        this.rows = 24;
        this.cols = 32;
        this.noteNames = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        this.defaultDuration = 8;

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
                this.currentInstrument = parseInt(e.target.value);
                this.tracks[this.currentTrackIndex].instrument = this.currentInstrument;
            });
        }

        const priorityInput = document.getElementById('audio-priority-input');
        if (priorityInput) {
            priorityInput.addEventListener('change', (e) => {
                const val = parseInt(e.target.value);
                this.tracks[this.currentTrackIndex].priority = isNaN(val) ? 0 : val;
            });
        }

        const btnPlay = document.getElementById('btn-play-track');
        if (btnPlay) {
            btnPlay.addEventListener('click', () => {
                console.log("Preview not implemented in JS yet. Compile to hear.");
            });
        }

        const btnSamples = document.getElementById('btn-manage-samples');
        if (btnSamples) {
            btnSamples.addEventListener('click', () => {
                const trackerRoot = document.getElementById('audio-tracker-root');
                const samplesRoot = document.getElementById('audio-samples-root');
                if (samplesRoot.style.display === 'none') {
                    samplesRoot.style.display = 'block';
                    trackerRoot.style.display = 'none';
                    this.renderSampleList();
                } else {
                    samplesRoot.style.display = 'none';
                    trackerRoot.style.display = 'grid';
                }
            });
        }

        const btnAddSample = document.getElementById('btn-add-sample');
        if (btnAddSample) {
             btnAddSample.addEventListener('click', () => this.importSample());
        }
    }

    renderGrid() {
        this.root.innerHTML = '';

        let startOctave = 4;
        let noteIndex = this.noteNames.length - 1;

        for (let r = 0; r < this.rows; r++) {
            const currentNote = this.noteNames[noteIndex];
            const isBlackKey = currentNote.includes('#');
            const labelText = `${currentNote}${startOctave}`;

            const label = document.createElement('div');
            label.className = `tracker-row-label ${isBlackKey ? 'black-key' : ''}`;
            label.textContent = labelText;
            this.root.appendChild(label);

            for (let c = 0; c < this.cols; c++) {
                const cell = document.createElement('div');
                cell.className = 'tracker-cell';
                cell.dataset.row = r;
                cell.dataset.col = c;
                cell.dataset.pitch = (startOctave * 12) + noteIndex;

                cell.addEventListener('mousedown', (e) => this.toggleNote(r, c, cell));

                this.root.appendChild(cell);
            }

            noteIndex--;
            if (noteIndex < 0) {
                noteIndex = this.noteNames.length - 1;
                startOctave--;
            }
        }
    }

    toggleNote(row, col, cellElement) {
        const track = this.tracks[this.currentTrackIndex];
        const existingIndex = track.notes.findIndex(n => n.row === row && n.col === col);

        if (existingIndex >= 0) {
            track.notes.splice(existingIndex, 1);
            cellElement.classList.remove('active');
        } else {
            track.notes.push({
                row: row,
                col: col,
                pitch: cellElement.dataset.pitch,
                duration: this.defaultDuration
            });
            cellElement.classList.add('active');
        }
    }

    loadTrackUI() {
        const cells = this.root.querySelectorAll('.tracker-cell');
        cells.forEach(c => c.classList.remove('active'));

        const instrumentSelect = document.getElementById('audio-instrument-select');
        if (instrumentSelect) {
            const trk = this.tracks[this.currentTrackIndex];
            if (trk.instrument === undefined) trk.instrument = 0x9F;
            instrumentSelect.value = trk.instrument;
        }

        const priorityInput = document.getElementById('audio-priority-input');
        if (priorityInput) {
            const trk = this.tracks[this.currentTrackIndex];
            if (trk.priority === undefined) trk.priority = 0;
            priorityInput.value = trk.priority;
        }

        const track = this.tracks[this.currentTrackIndex];
        track.notes.forEach(note => {
            const cell = this.root.querySelector(`.tracker-cell[data-row="${note.row}"][data-col="${note.col}"]`);
            if (cell) {
                cell.classList.add('active');
            }
        });
    }

    getData() {
        return {
            tracks: this.tracks,
            samples: this.samples
        };
    }

    loadData(data) {
        if (data && data.audio_tracks) {
            this.tracks = data.audio_tracks.map((t, i) => ({
                name: t.name || `Track ${i+1}`,
                notes: t.notes.map(n => ({
                    row: n.row,
                    col: n.col || n.step,
                    pitch: n.pitch,
                    duration: n.duration
                })),
                channel: (t.channel !== undefined) ? t.channel : i,
                instrument: (t.instrument !== undefined) ? t.instrument : ((i===2)?0xFF:((i===3)?0x0F:0x9F)),
                priority: (t.priority !== undefined) ? t.priority : 0
            }));

            // Fill up to 4 if missing
            while (this.tracks.length < 4) {
                let i = this.tracks.length;
                this.tracks.push({
                    name: `Track ${i+1}`,
                    notes: [],
                    channel: i,
                    instrument: (i===2)?0xFF:((i===3)?0x0F:0x9F),
                    priority: 0
                });
            }
        } else {
             this.tracks = [
                { name: "Track 1", notes: [], channel: 0, instrument: 0x9F, priority: 0 },
                { name: "Track 2", notes: [], channel: 1, instrument: 0x9F, priority: 0 },
                { name: "Track 3", notes: [], channel: 2, instrument: 0xFF, priority: 0 },
                { name: "Track 4", notes: [], channel: 3, instrument: 0x0F, priority: 0 }
            ];
        }

        if (data && data.samples) {
            this.samples = data.samples;
        } else {
            this.samples = [];
        }

        this.loadTrackUI();
    }

    async importSample() {
        const fileInput = document.getElementById('sample-upload');
        const nameInput = document.getElementById('sample-name');

        if (!fileInput.files.length) return;
        const file = fileInput.files[0];
        const name = nameInput.value || file.name.split('.')[0];

        const arrayBuffer = await file.arrayBuffer();
        const audioContext = new (window.AudioContext || window.webkitAudioContext)();
        const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);

        const dpcmData = this.encodeDPCM(audioBuffer);

        this.samples.push({
            name: name,
            data: Array.from(dpcmData)
        });

        this.renderSampleList();
        alert(`Imported sample "${name}" (${dpcmData.length} bytes)`);
    }

    encodeDPCM(audioBuffer) {
        const channelData = audioBuffer.getChannelData(0);
        let accumulator = 0;
        const outputBytes = [];
        let currentByte = 0;
        let bitIndex = 0;

        for (let i = 0; i < channelData.length; i++) {
            const target = Math.floor((channelData[i] + 1.0) * 63.5);
            const clampedTarget = Math.max(0, Math.min(127, target));

            let bit = 0;
            if (clampedTarget > accumulator) {
                bit = 1;
                if (accumulator <= 125) accumulator += 2;
            } else {
                bit = 0;
                if (accumulator >= 2) accumulator -= 2;
            }

            if (bit) {
                currentByte |= (1 << bitIndex);
            }

            bitIndex++;
            if (bitIndex === 8) {
                outputBytes.push(currentByte);
                currentByte = 0;
                bitIndex = 0;
            }
        }

        if (bitIndex > 0) {
            outputBytes.push(currentByte);
        }

        return new Uint8Array(outputBytes);
    }

    renderSampleList() {
        const list = document.getElementById('sample-list');
        list.innerHTML = '';
        this.samples.forEach((s, i) => {
            const li = document.createElement('li');
            li.textContent = `ID ${i}: ${s.name} (${s.data.length} bytes)`;

            const btnDelete = document.createElement('button');
            btnDelete.textContent = 'X';
            btnDelete.style.marginLeft = '10px';
            btnDelete.onclick = () => {
                this.samples.splice(i, 1);
                this.renderSampleList();
            };
            li.appendChild(btnDelete);

            list.appendChild(li);
        });
    }
}

window.AudioTracker = AudioTracker;

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
        this.envelopeEditor = null;

        // Configuration
        this.rows = 24;
        this.cols = 32;
        this.noteNames = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        this.defaultDuration = 8;

        this.initUI();
    }

    initUI() {
        this.envelopeEditor = new window.EnvelopeEditor('audio-envelopes-root');
        this.renderGrid();
        this.bindEvents();
    }

    bindEvents() {
        const trackSelect = document.getElementById('audio-track-select');
        const instrumentSelect = document.getElementById('audio-instrument-select');
        const volEnvSelect = document.getElementById('audio-vol-env-select');
        const pitchEnvSelect = document.getElementById('audio-pitch-env-select');
        const arpEnvSelect = document.getElementById('audio-arp-env-select');

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

        if (volEnvSelect) {
            volEnvSelect.addEventListener('change', (e) => {
                const val = parseInt(e.target.value);
                this.tracks[this.currentTrackIndex].vol_env = (val < 0) ? null : val;
            });
        }

        if (pitchEnvSelect) {
            pitchEnvSelect.addEventListener('change', (e) => {
                const val = parseInt(e.target.value);
                this.tracks[this.currentTrackIndex].pitch_env = (val < 0) ? null : val;
            });
        }

        if (arpEnvSelect) {
            arpEnvSelect.addEventListener('change', (e) => {
                const val = parseInt(e.target.value);
                this.tracks[this.currentTrackIndex].arpeggio_env = (val < 0) ? null : val;
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
                const envRoot = document.getElementById('audio-envelopes-root');

                envRoot.style.display = 'none';
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

        const btnEnvelopes = document.getElementById('btn-manage-envelopes');
        if (btnEnvelopes) {
            btnEnvelopes.addEventListener('click', () => {
                const trackerRoot = document.getElementById('audio-tracker-root');
                const samplesRoot = document.getElementById('audio-samples-root');
                const envRoot = document.getElementById('audio-envelopes-root');

                samplesRoot.style.display = 'none';
                if (envRoot.style.display === 'none') {
                    envRoot.style.display = 'block';
                    trackerRoot.style.display = 'none';
                } else {
                    envRoot.style.display = 'none';
                    trackerRoot.style.display = 'grid';
                    this.loadTrackUI(); // Reload selects
                }
            });
        }

        window.addEventListener('envelopes-updated', () => {
            this.loadTrackUI();
        });

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

        const track = this.tracks[this.currentTrackIndex];

        const instrumentSelect = document.getElementById('audio-instrument-select');
        if (instrumentSelect) {
            if (track.instrument === undefined) track.instrument = 0x9F;
            instrumentSelect.value = track.instrument;
        }

        const priorityInput = document.getElementById('audio-priority-input');
        if (priorityInput) {
            if (track.priority === undefined) track.priority = 0;
            priorityInput.value = track.priority;
        }

        const updateEnvSelect = (id, val) => {
            const sel = document.getElementById(id);
            if (sel && this.envelopeEditor) {
                sel.innerHTML = '<option value="-1">No Env</option>';
                this.envelopeEditor.getEnvelopes().forEach((env, i) => {
                    const opt = document.createElement('option');
                    opt.value = i;
                    opt.textContent = env.name;
                    sel.appendChild(opt);
                });
                sel.value = (val === null || val === undefined) ? -1 : val;
            }
        };

        updateEnvSelect('audio-vol-env-select', track.vol_env);
        updateEnvSelect('audio-pitch-env-select', track.pitch_env);
        updateEnvSelect('audio-arp-env-select', track.arpeggio_env);

        track.notes.forEach(note => {
            const cell = this.root.querySelector(`.tracker-cell[data-row="${note.row}"][data-col="${note.col}"]`);
            if (cell) {
                cell.classList.add('active');
            }
        });
    }

    getData() {
        return {
            audio_tracks: this.tracks, // Renamed to match struct key
            samples: this.samples,
            envelopes: this.envelopeEditor ? this.envelopeEditor.getEnvelopes() : []
        };
    }

    loadData(data) {
        if (data && data.envelopes && this.envelopeEditor) {
            this.envelopeEditor.loadEnvelopes(data.envelopes);
        }

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
                priority: (t.priority !== undefined) ? t.priority : 0,
                vol_env: t.vol_env,
                pitch_env: t.pitch_env,
                arpeggio_env: t.arpeggio_env
            }));

            // Fill up to 4 if missing
            while (this.tracks.length < 4) {
                let i = this.tracks.length;
                this.tracks.push({
                    name: `Track ${i+1}`,
                    notes: [],
                    channel: i,
                    instrument: (i===2)?0xFF:((i===3)?0x0F:0x9F),
                    priority: 0,
                    vol_env: null,
                    pitch_env: null,
                    arpeggio_env: null
                });
            }
        } else {
             this.tracks = [
                { name: "Track 1", notes: [], channel: 0, instrument: 0x9F, priority: 0, vol_env: null, pitch_env: null, arpeggio_env: null },
                { name: "Track 2", notes: [], channel: 1, instrument: 0x9F, priority: 0, vol_env: null, pitch_env: null, arpeggio_env: null },
                { name: "Track 3", notes: [], channel: 2, instrument: 0xFF, priority: 0, vol_env: null, pitch_env: null, arpeggio_env: null },
                { name: "Track 4", notes: [], channel: 3, instrument: 0x0F, priority: 0, vol_env: null, pitch_env: null, arpeggio_env: null }
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

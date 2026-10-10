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
                if (!this.tracks[this.currentTrackIndex]?.notes.length) {
                    alert('Add notes to this track before playing it.');
                    return;
                }
                // Preview uses the same compiled APU runtime as the game,
                // including DPCM and envelopes, rather than a second synth.
                window.editor?.runEmulator({previewTrackId: this.currentTrackIndex});
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

        window.addEventListener('envelopes-updated', event => {
            const removed = event.detail?.deletedIndex;
            if (Number.isInteger(removed)) {
                for (const track of this.tracks) {
                    for (const field of ['vol_env', 'pitch_env', 'arpeggio_env']) {
                        if (track[field] === removed) track[field] = null;
                        else if (track[field] > removed) track[field]--;
                    }
                }
            }
            this.loadTrackUI();
        });

        const btnAddSample = document.getElementById('btn-add-sample');
        if (btnAddSample) {
             btnAddSample.addEventListener('click', () => this.importSample());
        }
    }

    renderGrid() {
        this.root.innerHTML = '';

        const sampleTrack = this.tracks[this.currentTrackIndex]?.channel === 3;
        if (sampleTrack && this.samples.length === 0) {
            const hint = document.createElement('p');
            hint.textContent = 'Import a WAV sample to add DMC notes.';
            this.root.appendChild(hint);
            return;
        }
        const rows = sampleTrack ? this.samples.length : this.rows;
        this.root.style.gridTemplateRows = `repeat(${rows}, 20px)`;
        this.root.style.gridTemplateColumns = `${sampleTrack ? 120 : 40}px repeat(${this.cols}, 1fr)`;

        // The preserved period table begins at physical C2, so index48 is C6.
        let startOctave = 6;
        let noteIndex = this.noteNames.length - 1;

        for (let r = 0; r < rows; r++) {
            const currentNote = this.noteNames[noteIndex];
            const isBlackKey = currentNote.includes('#');
            const labelText = sampleTrack ? `${r}: ${this.samples[r].name}` : `${currentNote}${startOctave}`;

            const label = document.createElement('div');
            label.className = `tracker-row-label ${isBlackKey ? 'black-key' : ''}`;
            label.textContent = labelText;
            this.root.appendChild(label);

            for (let c = 0; c < this.cols; c++) {
                const cell = document.createElement('div');
                cell.className = 'tracker-cell';
                cell.dataset.row = r;
                cell.dataset.col = c;
                cell.dataset.pitch = sampleTrack ? r : (startOctave * 12) + noteIndex;

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
                pitch: Number(cellElement.dataset.pitch),
                duration: this.defaultDuration
            });
            cellElement.classList.add('active');
        }
    }

    loadTrackUI() {
        this.renderGrid();
        const cells = this.root.querySelectorAll('.tracker-cell');
        cells.forEach(c => c.classList.remove('active'));

        const track = this.tracks[this.currentTrackIndex];

        const instrumentSelect = document.getElementById('audio-instrument-select');
        if (instrumentSelect) {
            instrumentSelect.innerHTML = '';
            const choices = track.channel === 3
                ? Array.from({length: 16}, (_, rate) => [rate, `DMC rate ${rate}`])
                : track.channel === 2 ? [[0xff, 'Triangle linear counter on']]
                : [[0x9f, '50% duty, max volume'], [0xbf, '75% duty, max volume'],
                    [0x3f, '25% duty, max volume'], [0xcf, '75% duty, decay']];
            for (const [value, name] of choices) {
                const option = document.createElement('option');
                option.value = value;
                option.textContent = name;
                instrumentSelect.appendChild(option);
            }
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
            audio_tracks: this.tracks.map(track => ({...track, notes: track.notes.map(note => ({
                ...note, pitch: Number(note.pitch), row: Number(note.row ?? 0),
                col: Number(note.col ?? note.step ?? 0), duration: Number(note.duration)
            }))})),
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
                    row: n.row ?? 0,
                    col: n.col ?? n.step ?? 0,
                    pitch: Number(n.pitch),
                    duration: Number(n.duration)
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

        let audioContext;
        const project = window.projectManager?.currentProject;
        const samples = this.samples;
        try {
            if (!/\.wav$/i.test(file.name)) throw new Error('Choose a WAV file.');
            if (samples.length >= 64) throw new Error('At most 64 DPCM samples are supported.');
            const arrayBuffer = await file.arrayBuffer();
            audioContext = new (window.AudioContext || window.webkitAudioContext)();
            const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);
            const track = this.tracks[this.currentTrackIndex];
            const rate = track.channel === 3 ? track.instrument & 15 : 15;
            const dpcmData = this.encodeDPCM(audioBuffer, rate);
            if (project !== window.projectManager?.currentProject || samples !== this.samples) return;
            samples.push({name, data: Array.from(dpcmData)});
            this.renderSampleList();
            this.loadTrackUI();
            alert(`Imported sample "${name}" (${dpcmData.length} bytes, DMC rate ${rate})`);
        } catch (error) {
            alert('Sample import failed: ' + error.message);
        } finally {
            try { await audioContext?.close(); } catch (error) { console.warn('Audio context cleanup:', error); }
        }
    }

    encodeDPCM(audioBuffer, rate = 15) {
        const channelData = audioBuffer.getChannelData(0);
        // Resample to the chosen NTSC DMC bit clock so a 44.1/48kHz WAV
        // retains its duration at playback. The byte count is checked before
        // encoding; the hardware cannot describe more than 4081 bytes.
        const periods = [428,380,340,320,286,254,226,214,190,160,142,128,106,84,72,54];
        if (!Number.isInteger(rate) || rate < 0 || rate > 15) throw new Error('DMC rate must be 0..15.');
        const sourceRate = audioBuffer.sampleRate;
        if (!Number.isFinite(sourceRate) || sourceRate <= 0 || channelData.length === 0) {
            throw new Error('WAV has no valid audio samples.');
        }
        const targetRate = 1789773 / periods[rate];
        const bitCount = Math.ceil(channelData.length * targetRate / sourceRate);
        if (Math.ceil(bitCount / 8) > 4081) throw new Error('WAV is too long for a 4081-byte DPCM sample at this rate.');
        let accumulator = 0;
        const outputBytes = [];
        let currentByte = 0;
        let bitIndex = 0;

        for (let i = 0; i < bitCount; i++) {
            const position = i * sourceRate / targetRate;
            const low = Math.floor(position);
            const fraction = position - low;
            const first = channelData[Math.min(low, channelData.length - 1)];
            const next = channelData[Math.min(low + 1, channelData.length - 1)];
            if (!Number.isFinite(first) || !Number.isFinite(next)) throw new Error('WAV contains invalid samples.');
            const target = Math.floor((first + (next - first) * fraction + 1.0) * 63.5);
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
                // Keep remaining sample IDs stable in authored DMC notes.
                for (const track of this.tracks.filter(track => track.channel === 3)) {
                    track.notes = track.notes.filter(note => note.pitch !== i).map(note => {
                        const pitch = note.pitch > i && note.pitch !== 255 ? note.pitch - 1 : note.pitch;
                        return {...note, pitch, row: pitch};
                    });
                }
                this.renderSampleList();
                this.loadTrackUI();
            };
            li.appendChild(btnDelete);

            list.appendChild(li);
        });
    }
}

window.AudioTracker = AudioTracker;

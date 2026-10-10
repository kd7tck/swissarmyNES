
class SynthEngine {
    constructor() {
        this.ctx = new (window.AudioContext || window.webkitAudioContext)();
        this.masterGain = this.ctx.createGain();
        this.masterGain.gain.value = 0.5;
        this.analyser = this.ctx.createAnalyser();
        this.analyser.fftSize = 2048;

        this.masterGain.connect(this.analyser);
        this.analyser.connect(this.ctx.destination);

        this.isPlaying = false;
        this.timer = null;
        this.sfx = null;
        this.stepIndex = 0;
        this.lastDuty = null;
        this.samplesPerFrame = 0;
        this.nextStepTime = 0;

        // Current state
        this.osc = null;
        this.noiseNode = null;
        this.gainNode = null;

        // Cache PeriodicWaves for Pulse duties
        this.pulseWaves = [
            this.createPulseWave(0.125),
            this.createPulseWave(0.25),
            this.createPulseWave(0.50),
            this.createPulseWave(0.75)
        ];
    }

    createPulseWave(duty) {
        // Fourier series for pulse wave
        // We use a reasonable number of harmonics to avoid aliasing artifacts at high pitch,
        // but low enough to keep performance good.
        const numTerms = 256;
        const real = new Float32Array(numTerms);
        const imag = new Float32Array(numTerms);

        real[0] = 0;
        imag[0] = 0;

        for (let i = 1; i < numTerms; i++) {
            // Coeffs for pulse wave
            const n = i;
            real[n] = 0; // Sine terms only for odd function? Square is odd.
            // Pulse wave formula coeff: (2/(n*pi)) * sin(n * pi * duty)
            imag[n] = (2 / (n * Math.PI)) * Math.sin(n * Math.PI * duty);
        }
        return this.ctx.createPeriodicWave(real, imag);
    }

    createNoiseBuffer() {
        const bufferSize = this.ctx.sampleRate * 2; // 2 seconds
        const buffer = this.ctx.createBuffer(1, bufferSize, this.ctx.sampleRate);
        const data = buffer.getChannelData(0);
        for (let i = 0; i < bufferSize; i++) {
            data[i] = Math.random() * 2 - 1;
        }
        return buffer;
    }

    play(sfx) {
        this.stop();
        if (!Number.isInteger(sfx.channel) || sfx.channel < 0 || sfx.channel > 2) {
            throw new Error('SFX macros support Pulse 1, Pulse 2, and Triangle.');
        }
        this.lastDuty = null;
        if (this.ctx.state === 'suspended') this.ctx.resume();

        this.sfx = sfx;
        this.isPlaying = true;
        this.stepIndex = 0;
        this.nextStepTime = this.ctx.currentTime;

        // Frame duration (approx 60Hz)
        const frameTime = 1 / 60;
        const speed = Math.max(1, sfx.speed || 1);
        this.stepDuration = frameTime * speed;

        // Setup Nodes based on Channel
        this.gainNode = this.ctx.createGain();
        this.gainNode.connect(this.masterGain);

        // NTSC Frequency base
        // Tuning is approximate.
        this.baseFreq = 440;

        if (sfx.channel === 3) {
            // Noise
            this.noiseBuffer = this.createNoiseBuffer();
            this.noiseNode = this.ctx.createBufferSource();
            this.noiseNode.buffer = this.noiseBuffer;
            this.noiseNode.loop = true;
            this.noiseNode.connect(this.gainNode);
            this.noiseNode.start();
        } else if (sfx.channel === 2) {
            // Triangle
            this.osc = this.ctx.createOscillator();
            this.osc.type = 'triangle';
            this.osc.connect(this.gainNode);
            this.osc.start();
        } else {
            // Pulse
            this.osc = this.ctx.createOscillator();
            // Default duty 50%
            this.osc.setPeriodicWave(this.pulseWaves[2]);
            this.osc.connect(this.gainNode);
            this.osc.start();
        }

        this.scheduleNextStep();
    }

    stop() {
        this.isPlaying = false;
        if (this.timer) {
            clearTimeout(this.timer);
            this.timer = null;
        }
        if (this.osc) {
            this.osc.onended = null;
            this.osc.stop();
            this.osc.disconnect();
            this.osc = null;
        }
        if (this.noiseNode) {
            this.noiseNode.stop();
            this.noiseNode.disconnect();
            this.noiseNode = null;
        }
        if (this.gainNode) {
            this.gainNode.disconnect();
            this.gainNode = null;
        }
    }

    scheduleNextStep() {
        if (!this.isPlaying) return;

        const sfx = this.sfx;
        const now = this.ctx.currentTime;
        const sequenceLength = Math.max(sfx.vol_sequence.length, sfx.pitch_sequence.length, sfx.duty_sequence.length, (sfx.arpeggio_sequence || []).length);

        // Lookahead
        while (this.nextStepTime < now + 0.1) {
            if ((!sfx.does_loop || sequenceLength === 0) && this.stepIndex >= sequenceLength) {
                this.finishPreview(this.nextStepTime);
                return;
            }
            // Get Envelope Values
            // Vol
            const volLen = sfx.vol_sequence.length;
            let vol = 15;
            if (volLen > 0) {
                const idx = (sfx.does_loop && this.stepIndex >= volLen)
                    ? this.stepIndex % volLen
                    : Math.min(this.stepIndex, volLen - 1);
                vol = sfx.vol_sequence[idx];
            }

            // Pitch
            const pitchLen = sfx.pitch_sequence.length;
            let pitchShift = 0;
            if (pitchLen > 0) {
                const idx = (sfx.does_loop && this.stepIndex >= pitchLen)
                    ? this.stepIndex % pitchLen
                    : Math.min(this.stepIndex, pitchLen - 1);
                pitchShift = sfx.pitch_sequence[idx];
            }

            // Duty
            const dutyLen = sfx.duty_sequence.length;
            let duty = 0;
            if (dutyLen > 0) {
                const idx = (sfx.does_loop && this.stepIndex >= dutyLen)
                    ? this.stepIndex % dutyLen
                    : Math.min(this.stepIndex, dutyLen - 1);
                duty = sfx.duty_sequence[idx];
            }

            // Apply Updates
            const time = this.nextStepTime;

            // Volume (0-15)
            // Scale to 0.0 - 1.0 (Linear-ish)
            this.gainNode.gain.setValueAtTime(vol / 15.0, time);

            // Pitch & Duty
            // Match the runtime's note-48 base, semitone arpeggio, and signed timer
            // offset. Each sequence step is relative to the base rather than cumulative.
            const arpeggio = sfx.arpeggio_sequence || [];
            const arpIndex = sfx.does_loop ? this.stepIndex % arpeggio.length : Math.min(this.stepIndex, arpeggio.length - 1);
            const note = Math.max(0, Math.min(95, 48 + (arpeggio[arpIndex] || 0)));
            const divisor = sfx.channel === 2 ? 32 : 16;
            const basePeriod = SynthEngine.notePeriod(note);
            this.currentPeriod = Math.max(0, Math.min(2047, basePeriod + pitchShift));
            const freq = 1789773 / (divisor * (this.currentPeriod + 1));

            if (this.osc) {
                this.osc.frequency.setValueAtTime(freq, time);
                // Duty
                if (sfx.channel === 0 || sfx.channel === 1) {
                    const d = Math.max(0, Math.min(3, duty));
                    // We can't schedule setPeriodicWave. We have to do it now?
                    // setPeriodicWave cannot be scheduled. It's immediate.
                    // This might cause artifacts if we are ahead of time.
                    // For preview, doing it immediately is okay-ish, or we use a Timeout.
                    // We'll skip precise scheduling for duty wave shape.
                    if (this.lastDuty !== d) {
                        this.osc.setPeriodicWave(this.pulseWaves[d]);
                        this.lastDuty = d;
                    }
                }
            } else if (this.noiseNode) {
                // Noise Pitch controls playback rate
                // NES Noise periods: 4, 8, 16, 32...
                // We map currentPeriod to a playback rate?
                // Simpler: Map 'freq' to playbackRate.
                // 44100Hz = rate 1.0.
                // Noise freqs range from 31kHz to ~30Hz.
                const rate = freq / 1000; // Rough approximation
                this.noiseNode.playbackRate.setValueAtTime(rate, time);
            }

            this.stepIndex++;
            this.nextStepTime += this.stepDuration;
        }

        this.timer = setTimeout(() => this.scheduleNextStep(), 20);
    }

    finishPreview(time) {
        this.isPlaying = false;
        if (this.timer) clearTimeout(this.timer);
        this.timer = null;
        const source = this.osc || this.noiseNode;
        const gain = this.gainNode;
        if (!source) return;
        // Lookahead has already queued future steps. Stop at their end rather than
        // disconnecting now, which would silence every short preview immediately.
        source.onended = () => {
            source.disconnect();
            gain.disconnect();
            if (this.osc === source) this.osc = null;
            if (this.noiseNode === source) this.noiseNode = null;
            if (this.gainNode === gain) this.gainNode = null;
        };
        source.stop(time);
    }

    getAnalyser() {
        return this.analyser;
    }

    static notePeriod(note) {
        // The compiler emits this same NTSC timer table at $D000. Matching integer
        // timers keeps the authoring preview faithful to encoded semitone steps.
        const periods = [
            0x6AE, 0x64E, 0x5F4, 0x59E, 0x54D, 0x501, 0x4B8, 0x474, 0x432, 0x3F4, 0x3B8, 0x380,
            0x357, 0x327, 0x2FA, 0x2CF, 0x2A6, 0x280, 0x25C, 0x23A, 0x219, 0x1FA, 0x1DC, 0x1C0,
            0x1AB, 0x193, 0x17D, 0x167, 0x153, 0x140, 0x12E, 0x11D, 0x10C, 0x0FD, 0x0EE, 0x0E0,
            0x0D5, 0x0C9, 0x0BE, 0x0B3, 0x0A9, 0x0A0, 0x097, 0x08E, 0x086, 0x07E, 0x077, 0x070,
            0x06A, 0x064, 0x05F, 0x059, 0x054, 0x050, 0x04B, 0x047, 0x043, 0x03F, 0x03B, 0x038,
            0x035, 0x032, 0x02F, 0x02C, 0x02A, 0x028, 0x025, 0x023, 0x021, 0x01F, 0x01D, 0x01C,
            0x01A, 0x019, 0x017, 0x016, 0x015, 0x014, 0x012, 0x011, 0x010, 0x00F, 0x00E, 0x00E,
            0x00D, 0x00C, 0x00B, 0x00B, 0x00A, 0x00A, 0x009, 0x008, 0x008, 0x007, 0x007, 0x007
        ];
        return periods[Math.max(0, Math.min(95, note))];
    }
}

window.SynthEngine = SynthEngine;

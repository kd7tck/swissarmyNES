
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

        // Lookahead
        while (this.nextStepTime < now + 0.1) {
            // Get Envelope Values
            // Vol
            const volLen = sfx.vol_sequence.length;
            let vol = 0;
            if (volLen > 0) {
                const idx = (sfx.does_loop && this.stepIndex >= volLen)
                    ? this.stepIndex % volLen
                    : Math.min(this.stepIndex, volLen - 1);
                vol = sfx.vol_sequence[idx];
            }
            // If sequence ended and no loop, and we are past end, silence?
            // The NES engine holds the last value if no loop terminator, or 0 if 0-terminated.
            // Our editor data model doesn't explicitly store the terminator byte, we assume array end.
            // Usually, vol envelope ends with 0.
            if (!sfx.does_loop && this.stepIndex >= volLen) {
                 vol = 0; // Default to silence at end of vol envelope
                 if (this.stepIndex > Math.max(sfx.pitch_sequence.length, sfx.duty_sequence.length) + 10) {
                     // Auto-stop after some padding
                     this.stop();
                     return;
                 }
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
            // Base pitch? SFX are usually relative pitch sweeps.
            // We need a base note. Let's assume Middle C equivalent or some fixed frequency.
            // NES periods are inverse freq.
            // Let's use 440Hz as base.
            // Pitch shift is usually semitones or raw period units.
            // In NES engine, it's raw period units (subtracted).
            // -1 Period unit is Higher frequency.
            // Approx: F = 111860.8 / (Period + 1).
            // Let's assume a base period of 200 (~556Hz).
            // Shift is added to period.
            // NewPeriod = Base - (pitchShift * 4) // Scaling for effect

            // Actually, sfx.pitch_sequence is i8. Positive adds to period (lower pitch), Negative subtracts (higher).
            // Let's accumulate pitch if it's a sweep?
            // The NES engine accumulates: `ADC $0A` (Current Pitch) + `ADC $F0` (Envelope Val).
            // So it IS a relative delta applied every frame.
            // We need to track current period.
            if (this.stepIndex === 0) {
                this.currentPeriod = 200; // Start Period
            }
            this.currentPeriod = Math.max(10, Math.min(2000, this.currentPeriod + pitchShift));

            // Convert Period to Freq
            // Freq = CPU / (16 * (P + 1))
            const freq = 1789773 / (16 * (this.currentPeriod + 1));

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

    getAnalyser() {
        return this.analyser;
    }
}

window.SynthEngine = SynthEngine;

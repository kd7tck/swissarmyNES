// Basic Syntax Highlighter and Line Number Manager

class SwissEditor {
    constructor(editorId, highlightId, lineNumbersId) {
        this.editor = document.getElementById(editorId);
        this.highlight = document.getElementById(highlightId);
        this.lineNumbers = document.getElementById(lineNumbersId);
        this.emulator = null; // Store emulator instance
        this.emulatorRunning = false;
        this.audioContext = null;
        this.nextStartTime = 0;
        this.volume = 0.5;
        this.scale = 2; // 1, 2, or 3 (Full)
        this.wasmLoaded = false;
        this.wasmMemory = null;

        // Input State
        this.gamepadIndex = null;
        this.keyboardState = new Array(8).fill(false);
        this.gamepadState = new Array(8).fill(false);
        this.lastSentState = new Array(8).fill(false);

        this.init();
    }

    init() {
        if (!this.editor) return;

        // Bind events
        this.editor.addEventListener('input', () => this.update());
        this.editor.addEventListener('scroll', () => this.syncScroll());
        this.editor.addEventListener('keydown', (e) => this.handleKey(e));

        // Initial update
        this.update();

        // Bind Run button
        const btnRun = document.getElementById('btn-run');
        if (btnRun) {
            btnRun.addEventListener('click', () => this.runEmulator());
        }

        // Listen for compile event
        window.addEventListener('emulator-load-rom', (e) => {
             this.startEmulatorWithRom(e.detail);
        });

        // Gamepad Events
        window.addEventListener("gamepadconnected", (e) => {
            console.log("Gamepad connected at index %d: %s. %d buttons, %d axes.",
                e.gamepad.index, e.gamepad.id,
                e.gamepad.buttons.length, e.gamepad.axes.length);
            if (this.gamepadIndex === null) {
                this.gamepadIndex = e.gamepad.index;
            }
        });

        window.addEventListener("gamepaddisconnected", (e) => {
            console.log("Gamepad disconnected from index %d: %s",
                e.gamepad.index, e.gamepad.id);
            if (this.gamepadIndex === e.gamepad.index) {
                this.gamepadIndex = null;
            }
        });
    }

    update() {
        const text = this.editor.value;
        this.updateLineNumbers(text);
        this.updateHighlighting(text);
    }

    syncScroll() {
        this.highlight.scrollTop = this.editor.scrollTop;
        this.highlight.scrollLeft = this.editor.scrollLeft;
        this.lineNumbers.scrollTop = this.editor.scrollTop;
    }

    handleKey(e) {
        if (e.key === 'Tab') {
            e.preventDefault();
            const start = this.editor.selectionStart;
            const end = this.editor.selectionEnd;
            this.editor.value = this.editor.value.substring(0, start) +
                "  " + this.editor.value.substring(end);
            this.editor.selectionStart = this.editor.selectionEnd = start + 2;
            this.update();
        }
    }

    updateLineNumbers(text) {
        const lines = text.split('\n').length;
        this.lineNumbers.innerHTML = Array(lines).fill(0).map((_, i) => i + 1).join('<br>');
    }

    updateHighlighting(text) {
        const lines = text.split('\n');
        const highlightedLines = lines.map(line => this.highlightLine(line));
        if (text.endsWith('\n')) highlightedLines.push('');
        this.highlight.innerHTML = highlightedLines.join('<br>');
    }

    highlightLine(line) {
        if (!line) return '';
        const keywords = [
            'REM', 'BEGIN', 'END', 'NEXT', 'WEND', 'IF', 'THEN', 'ELSE',
            'SUB', 'INTERRUPT', 'ASM', 'ON', 'AS', 'DO', 'WHILE', 'FOR',
            'TO', 'STEP', 'LOOP', 'CONST', 'DIM', 'BYTE', 'WORD', 'BOOL',
            'PEEK', 'POKE', 'PRINT', 'RETURN', 'CALL', 'AND', 'OR', 'NOT',
            'LET', 'PLAY_SFX', 'DATA', 'READ', 'RESTORE', 'TYPE', 'ENUM',
            'SELECT', 'CASE', 'MACRO', 'METASPRITE', 'TILE', 'ANIMATION',
            'FRAME', 'WAIT_VBLANK', 'Include', 'Sprite', 'Text', 'Controller',
            'Scroll', 'PPU', 'Random', 'Collision', 'Math'
        ];
        const keywordSet = new Set(keywords.map(k => k.toUpperCase()));
        const tokenRegex = /(".*?")|('.*)|(\$[0-9A-Fa-f]+|%[01]+)|([a-zA-Z0-9_]+)|(.)/g;
        let resultHtml = '';
        let match;

        while ((match = tokenRegex.exec(line)) !== null) {
            const [full, str, comment, number, word, other] = match;
            if (str) {
                resultHtml += `<span class="hl-string">${this.escapeHtml(str)}</span>`;
            } else if (comment) {
                resultHtml += `<span class="hl-comment">${this.escapeHtml(comment)}</span>`;
            } else if (number) {
                 resultHtml += `<span class="hl-number">${this.escapeHtml(number)}</span>`;
            } else if (word) {
                const upper = word.toUpperCase();
                if (upper === 'REM') {
                     resultHtml += `<span class="hl-comment">${this.escapeHtml(word)}</span>`;
                     const rest = line.substring(tokenRegex.lastIndex);
                     resultHtml += `<span class="hl-comment">${this.escapeHtml(rest)}</span>`;
                     break;
                } else if (keywordSet.has(upper)) {
                     resultHtml += `<span class="hl-keyword">${this.escapeHtml(word)}</span>`;
                } else if (/^[0-9]+$/.test(word)) {
                     resultHtml += `<span class="hl-number">${this.escapeHtml(word)}</span>`;
                } else {
                     resultHtml += this.escapeHtml(word);
                }
            } else if (other) {
                resultHtml += this.escapeHtml(other);
            }
        }
        return resultHtml;
    }

    escapeHtml(unsafe) {
        return unsafe.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;").replace(/'/g, "&#039;");
    }

    async runEmulator() {
        console.log("Starting emulator...");
        if (!this.wasmLoaded) {
            try {
                const module = await import('/wasm/swiss_emulator.js');
                const initResult = await module.default();
                this.EmulatorClass = module.Emulator;

                if (initResult && initResult.memory) {
                    this.wasmMemory = initResult.memory;
                } else if (module.memory) {
                    this.wasmMemory = module.memory;
                } else if (window.wasm_bindgen && window.wasm_bindgen.memory) {
                    this.wasmMemory = window.wasm_bindgen.memory;
                }

                this.wasmLoaded = true;
            } catch (e) {
                console.error("Failed to load WASM:", e);
                alert("Failed to load emulator core.");
                return;
            }
        }

        // Request compilation from app.js
        const event = new CustomEvent('request-compile-and-run');
        window.dispatchEvent(event);
    }

    startEmulatorWithRom(romData) {
        if (!this.EmulatorClass) return;

        // Init Audio
        if (!this.audioContext) {
            this.audioContext = new (window.AudioContext || window.webkitAudioContext)();
        }
        if (this.audioContext.state === 'suspended') {
            this.audioContext.resume();
        }

        try {
            if (this.emulator) {
                if (this.emulator.free) this.emulator.free();
            }
            this.emulator = new this.EmulatorClass();
            this.emulator.load_rom(romData);
            this.emulator.set_sample_rate(this.audioContext.sampleRate);
            this.nextStartTime = this.audioContext.currentTime;

            // Reset Input State
            this.keyboardState.fill(false);
            this.gamepadState.fill(false);
            this.lastSentState.fill(false);

            this.createEmulatorOverlay();

            // Start Loop
            this.emulatorRunning = true;
            this.updatePlayPauseButton();
            requestAnimationFrame(() => this.emulatorLoop());
        } catch (e) {
            console.error("Emulator error:", e);
            alert("Emulator crashed: " + e);
        }
    }

    createEmulatorOverlay() {
        let overlay = document.getElementById('emulator-overlay');
        if (!overlay) {
            overlay = document.createElement('div');
            overlay.id = 'emulator-overlay';
            Object.assign(overlay.style, {
                position: 'fixed', top: '0', left: '0', width: '100vw', height: '100vh',
                backgroundColor: 'rgba(0,0,0,0.9)', display: 'flex', flexDirection: 'column',
                justifyContent: 'center', alignItems: 'center', zIndex: '1000'
            });

            // Toolbar
            const toolbar = document.createElement('div');
            toolbar.style.marginBottom = '10px';
            toolbar.style.display = 'flex';
            toolbar.style.gap = '10px';
            toolbar.style.backgroundColor = '#333';
            toolbar.style.padding = '10px';
            toolbar.style.borderRadius = '5px';

            const btnPlay = document.createElement('button');
            btnPlay.id = 'emu-play';
            btnPlay.innerText = 'Pause';
            btnPlay.onclick = () => this.togglePause();

            const btnReset = document.createElement('button');
            btnReset.innerText = 'Reset';
            btnReset.onclick = () => { if(this.emulator) this.emulator.reset(); };

            const btn1x = document.createElement('button');
            btn1x.innerText = '1x';
            btn1x.onclick = () => this.setScale(1);

            const btn2x = document.createElement('button');
            btn2x.innerText = '2x';
            btn2x.onclick = () => this.setScale(2);

            const btnFull = document.createElement('button');
            btnFull.innerText = 'Full';
            btnFull.onclick = () => this.setScale(0);

            // Volume
            const volContainer = document.createElement('div');
            volContainer.style.display = 'flex';
            volContainer.style.alignItems = 'center';
            volContainer.style.color = '#fff';
            volContainer.innerHTML = 'Vol: ';
            const volSlider = document.createElement('input');
            volSlider.type = 'range';
            volSlider.min = 0; volSlider.max = 1; volSlider.step = 0.1;
            volSlider.value = this.volume;
            volSlider.oninput = (e) => { this.volume = parseFloat(e.target.value); };
            volContainer.appendChild(volSlider);

            const btnClose = document.createElement('button');
            btnClose.innerText = 'Close';
            btnClose.style.backgroundColor = '#d9534f';
            btnClose.onclick = () => {
                this.emulatorRunning = false;
                overlay.style.display = 'none';
                if(this.audioContext) this.audioContext.suspend();
            };

            toolbar.append(btnPlay, btnReset, btn1x, btn2x, btnFull, volContainer, btnClose);
            overlay.appendChild(toolbar);

            const canvas = document.createElement('canvas');
            canvas.id = 'emulator-canvas';
            canvas.width = 256;
            canvas.height = 240;
            canvas.style.imageRendering = 'pixelated';
            canvas.style.border = '2px solid #fff';

            overlay.appendChild(canvas);
            document.body.appendChild(overlay);
        } else {
            overlay.style.display = 'flex';
        }

        this.canvas = document.getElementById('emulator-canvas');
        this.ctx = this.canvas.getContext('2d');
        this.imageData = this.ctx.createImageData(256, 240);
        this.setScale(this.scale);

        // Bind keys
        document.addEventListener('keydown', (e) => this.handleEmulatorInput(e, true));
        document.addEventListener('keyup', (e) => this.handleEmulatorInput(e, false));
    }

    togglePause() {
        this.emulatorRunning = !this.emulatorRunning;
        this.updatePlayPauseButton();
        if (this.emulatorRunning) {
             if(this.audioContext && this.audioContext.state === 'suspended') this.audioContext.resume();
             this.emulatorLoop();
        } else {
             if(this.audioContext) this.audioContext.suspend();
        }
    }

    updatePlayPauseButton() {
        const btn = document.getElementById('emu-play');
        if(btn) btn.innerText = this.emulatorRunning ? 'Pause' : 'Play';
    }

    setScale(s) {
        this.scale = s;
        if (!this.canvas) return;
        if (s === 0) {
            this.canvas.style.width = '100vw';
            this.canvas.style.height = '100vh';
            this.canvas.style.objectFit = 'contain';
        } else {
            this.canvas.style.width = (256 * s) + 'px';
            this.canvas.style.height = (240 * s) + 'px';
            this.canvas.style.objectFit = 'unset';
        }
    }

    handleEmulatorInput(e, pressed) {
        if (document.getElementById('emulator-overlay').style.display === 'none' || !this.emulator) return;

        let btn = -1;
        switch(e.code) {
            case 'KeyZ': btn = 0; break; // A
            case 'KeyX': btn = 1; break; // B
            case 'ShiftLeft':
            case 'ShiftRight': btn = 2; break; // Select
            case 'Enter': btn = 3; break; // Start
            case 'ArrowUp': btn = 4; break;
            case 'ArrowDown': btn = 5; break;
            case 'ArrowLeft': btn = 6; break;
            case 'ArrowRight': btn = 7; break;
        }

        if (btn !== -1) {
            e.preventDefault();
            this.keyboardState[btn] = pressed;
            this.syncInputs();
        }
    }

    pollGamepads() {
        if (!this.emulator) return;

        const gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
        if (!gamepads) return;

        if (this.gamepadIndex === null) {
            for (let i = 0; i < gamepads.length; i++) {
                if (gamepads[i]) {
                    this.gamepadIndex = i;
                    break;
                }
            }
        }

        if (this.gamepadIndex === null) return;
        const gp = gamepads[this.gamepadIndex];
        if (!gp || !gp.connected) return;

        const b = gp.buttons;
        const axes = gp.axes;
        const PRESSED = (btn) => btn && (typeof btn === 'object' ? btn.pressed : btn > 0.5);

        // Map to NES: A, B, Sel, Start, Up, Down, Left, Right
        // A=0, B=1 or 2, Sel=8, Start=9
        const newState = [
            PRESSED(b[0]), // A
            PRESSED(b[1]) || PRESSED(b[2]), // B
            PRESSED(b[8]), // Select
            PRESSED(b[9]), // Start
            PRESSED(b[12]) || (axes.length > 1 && axes[1] < -0.5), // Up
            PRESSED(b[13]) || (axes.length > 1 && axes[1] > 0.5),  // Down
            PRESSED(b[14]) || (axes.length > 0 && axes[0] < -0.5), // Left
            PRESSED(b[15]) || (axes.length > 0 && axes[0] > 0.5)   // Right
        ];

        let changed = false;
        for (let i=0; i<8; i++) {
            if (this.gamepadState[i] !== newState[i]) {
                this.gamepadState[i] = newState[i];
                changed = true;
            }
        }
        if (changed) this.syncInputs();
    }

    syncInputs() {
        if (!this.emulator) return;
        for (let i = 0; i < 8; i++) {
            const pressed = this.keyboardState[i] || this.gamepadState[i];
            if (pressed !== this.lastSentState[i]) {
                this.emulator.set_button(0, i, pressed);
                this.lastSentState[i] = pressed;
            }
        }
    }

    emulatorLoop() {
        if (!this.emulatorRunning) return;

        // Poll inputs
        this.pollGamepads();

        try {
            this.emulator.step();

            // Render Video
            const pixelsPtr = this.emulator.get_pixels();
            const len = this.emulator.get_pixels_len();

            if (this.wasmMemory) {
                const pixels = new Uint8Array(this.wasmMemory.buffer, pixelsPtr, len);

                // Copy to ImageData
                if (len === 256 * 240 * 4) {
                    this.imageData.data.set(pixels);
                } else if (len === 256 * 240) {
                     // 8-bit palette - grayscale fallback
                     for (let i = 0; i < len; i++) {
                         const val = pixels[i];
                         const off = i * 4;
                         this.imageData.data[off] = val;     // R
                         this.imageData.data[off+1] = val;   // G
                         this.imageData.data[off+2] = val;   // B
                         this.imageData.data[off+3] = 255;   // A
                     }
                } else {
                    if (len === 184320) {
                        for (let i = 0; i < 256*240; i++) {
                             const off = i * 4;
                             const src = i * 3;
                             this.imageData.data[off] = pixels[src];
                             this.imageData.data[off+1] = pixels[src+1];
                             this.imageData.data[off+2] = pixels[src+2];
                             this.imageData.data[off+3] = 255;
                        }
                    }
                }

                this.ctx.putImageData(this.imageData, 0, 0);
            }

            // Audio
            const audioPtr = this.emulator.get_audio_samples();
            const audioLen = this.emulator.get_audio_samples_len();
            if (audioLen > 0 && this.wasmMemory && this.audioContext && this.audioContext.state === 'running') {
                 const samples = new Float32Array(this.wasmMemory.buffer, audioPtr, audioLen);

                 // Play audio
                 const buffer = this.audioContext.createBuffer(1, audioLen, this.audioContext.sampleRate);
                 buffer.copyToChannel(samples, 0);

                 const source = this.audioContext.createBufferSource();
                 source.buffer = buffer;

                 const gain = this.audioContext.createGain();
                 gain.gain.value = this.volume;

                 source.connect(gain);
                 gain.connect(this.audioContext.destination);

                 if (this.nextStartTime < this.audioContext.currentTime) {
                     this.nextStartTime = this.audioContext.currentTime;
                 }
                 source.start(this.nextStartTime);
                 this.nextStartTime += buffer.duration;

                 this.emulator.clear_audio_samples();
            }

            requestAnimationFrame(() => this.emulatorLoop());
        } catch (e) {
            console.error(e);
            this.emulatorRunning = false;
        }
    }
}

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
    window.editor = new SwissEditor('code-editor', 'code-highlight', 'line-numbers');

    // Set some default text if empty
    const editorEl = document.getElementById('code-editor');
    if(editorEl && !editorEl.value) {
        editorEl.value = "CONST BG_COLOR = $0F\n\nSUB Main()\n  ' Set Background Color\n  POKE($2006, $3F)\n  POKE($2006, $00)\n  POKE($2007, BG_COLOR)\n  \n  DO\n    WAIT_VBLANK\n  LOOP WHILE 1\nEND SUB";
        window.editor.update();
    }
});

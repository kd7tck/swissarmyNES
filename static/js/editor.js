// Basic Syntax Highlighter and Line Number Manager

// Removed import { PpuViewer } from './ppu_viewer.js'; as editor.js is not a module in HTML
// But PpuViewer is loaded via script tag before editor.js

class SwissEditor {
    constructor(editorId, highlightId, lineNumbersId) {
        this.editor = document.getElementById(editorId);
        this.highlight = document.getElementById(highlightId);
        this.lineNumbers = document.getElementById(lineNumbersId);
        this.emulator = null; // Store emulator instance
        this.emulatorRunning = false;
        this.audioContext = null;
        this.audioSources = new Set();
        this.nextStartTime = 0;
        this.volume = 0.5;
        this.scale = 2; // 1, 2, or 3 (Full)
        this.wasmLoaded = false;
        this.wasmMemory = null;
        this.frameCount = 0;
        this.frameRequest = null;
        this.lastFrameTime = null;
        this.frameDebt = 0;

        // Debug State
        this.sourceMap = { version: 1, sources: {}, entries: [] };
        this.compiledProject = null;
        this.activeLine = -1;
        this.breakpoints = new Set(); // Set of line numbers (1-based)
        this.breakpointFiles = new Map();
        this.observedSources = new Map();

        // Memory Viewer State
        this.memoryViewerOpen = false;
        this.memoryStart = 0x0000;
        this.memorySize = 0x0800; // 2KB Internal RAM

        // PPU Viewer State
        if (typeof PpuViewer !== 'undefined') {
            this.ppuViewer = new PpuViewer();
        } else {
            console.warn("PpuViewer not defined");
            this.ppuViewer = { attach:()=>{}, show:()=>{}, hide:()=>{}, update:()=>{} }; // dummy
        }

        // Input State
        this.gamepadIndex = null;
        this.keyboardState = new Array(8).fill(false);
        this.gamepadState = new Array(8).fill(false);
        this.lastSentState = new Array(8).fill(false);
        this.pressedKeys = new Set();

        this.init();
    }

    init() {
        if (!this.editor) return;

        // Bind events
        this.editor.addEventListener('input', () => { this.update(); this.syncBreakpoints(); this.clearDebugHighlight(); this.hideAddressTooltip(); });
        this.editor.addEventListener('scroll', () => this.syncScroll());
        this.editor.addEventListener('keydown', (e) => this.handleKey(e));
        // Global input handlers belong to the editor, not each reopening of the overlay.
        document.addEventListener('keydown', (e) => this.handleEmulatorInput(e, true));
        document.addEventListener('keyup', (e) => this.handleEmulatorInput(e, false));
        window.addEventListener('project-loaded', () => {
            this.sourceMap = { version: 1, sources: {}, entries: [] };
            this.breakpoints.clear();
            this.breakpointFiles.clear();
            this.observedSources.clear();
            this.syncBreakpoints();
            this.clearDebugHighlight();
            this.update();
        });
        window.addEventListener('blur', () => this.releaseInputs());
        document.addEventListener('visibilitychange', () => {
            if (document.hidden) {
                this.releaseInputs();
                if (this.emulatorRunning) this.togglePause();
            }
        });

        // Breakpoint toggle
        this.lineNumbers.addEventListener('click', (e) => {
            if (e.target.classList.contains('line-num')) {
                const line = parseInt(e.target.dataset.line);
                this.toggleBreakpoint(line);
            }
        });

        // Hover for Address (Phase 35)
        this.lineNumbers.addEventListener('mouseover', (e) => {
            if (e.target.classList.contains('line-num')) {
                const line = parseInt(e.target.dataset.line);
                this.showAddressTooltip(line, e.clientX, e.clientY);
            }
        });
        this.lineNumbers.addEventListener('mouseout', () => {
            this.hideAddressTooltip();
        });

        // Initial update
        this.update();

        // Address Tooltip Element
        this.tooltip = document.createElement('div');
        this.tooltip.id = 'address-tooltip';
        Object.assign(this.tooltip.style, {
            position: 'fixed', display: 'none', background: '#333', color: '#fff',
            padding: '4px', borderRadius: '4px', fontSize: '12px', zIndex: '2000',
            pointerEvents: 'none', border: '1px solid #666'
        });
        document.body.appendChild(this.tooltip);

        // Bind Run button
        const btnRun = document.getElementById('btn-run');
        if (btnRun) {
            btnRun.addEventListener('click', () => this.runEmulator());
        }

        // Listen for compile event
        window.addEventListener('emulator-load-rom', (e) => {
             // e.detail has { romData, sourceMap }
             if (e.detail.romData && e.detail.sourceMap) {
                 this.startEmulatorWithRom(e.detail.romData, e.detail.sourceMap);
             } else if (e.detail instanceof Uint8Array) {
                 // Legacy support (just rom data)
                 this.startEmulatorWithRom(e.detail, []);
             } else {
                 this.startEmulatorWithRom(e.detail.romData || e.detail, e.detail.sourceMap || []);
             }
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
                this.gamepadState.fill(false);
                this.syncInputs();
            }
        });
    }

    update() {
        const file = window.projectManager?.currentFile || 'main.swiss';
        if (!this.breakpointFiles.has(file)) this.breakpointFiles.set(file, new Set());
        this.breakpoints = this.breakpointFiles.get(file);
        this.observedSources.set(file, this.editor.value);
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
        // Use divs for line numbers so we can target them for debugging
        this.lineNumbers.innerHTML = Array(lines).fill(0)
            .map((_, i) => {
                const line = i + 1;
                const active = this.breakpoints.has(line) ? ' breakpoint' : '';
                return `<div class="line-num${active}" data-line="${line}">${line}</div>`;
            })
            .join('');
    }

    mappedEntries() {
        if (this.compiledProject !== (window.projectManager?.currentProject || null)) return [];
        const file = window.projectManager?.currentFile || 'main.swiss';
        if (this.sourceMap?.version !== 1 || this.sourceMap.sources?.[file] !== this.editor.value) return [];
        return (this.sourceMap.entries || []).filter(entry => entry.file === file && entry.kind === 'instruction');
    }

    showAddressTooltip(line, x, y) {
        const entry = this.mappedEntries().find(entry => entry.line === line);
        this.tooltip.innerText = entry
            ? `Bank ${entry.bank}: $${entry.cpu_start.toString(16).toUpperCase().padStart(4, '0')}`
            : 'No executable address';
        this.tooltip.style.left = (x + 10) + 'px';
        this.tooltip.style.top = (y + 10) + 'px';
        this.tooltip.style.display = 'block';
    }
    hideAddressTooltip() {
        if (this.tooltip) this.tooltip.style.display = 'none';
    }

    toggleBreakpoint(line) {
        if (this.breakpoints.has(line)) {
            this.breakpoints.delete(line);
        } else {
            this.breakpoints.add(line);
        }

        // Update UI
        const el = this.lineNumbers.querySelector(`div[data-line="${line}"]`);
        if (el) {
            el.classList.toggle('breakpoint');
        }

        // Update Emulator
        this.syncBreakpoints();
    }

    syncBreakpoints() {
        if (!this.emulator) return;
        this.emulator.clear_breakpoints();
        if (this.compiledProject !== (window.projectManager?.currentProject || null) || this.sourceMap?.version !== 1) return;
        // Keep included-file breakpoints armed when the explorer changes selection.
        // Edited snapshots are excluded until a new ROM/map pair is compiled.
        for (const [file, lines] of this.breakpointFiles) {
            if (this.observedSources.get(file) !== this.sourceMap.sources?.[file]) continue;
            for (const line of lines) {
                const entries = (this.sourceMap.entries || []).filter(entry => entry.file === file && entry.line === line && entry.kind === 'instruction');
                // A source line may expand into more than one bank/range.
                for (const entry of entries) this.emulator.add_breakpoint(entry.bank, entry.cpu_start);
            }
        }
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

    async runEmulator(options = {}) {
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
                // Expose globally for ppu viewer
                window.wasmMemory = this.wasmMemory;
            } catch (e) {
                console.error("Failed to load WASM:", e);
                alert("Failed to load emulator core.");
                return;
            }
        }

        // Request compilation from app.js
        const event = new CustomEvent('request-compile-and-run', {detail: options});
        window.dispatchEvent(event);
    }

    startEmulatorWithRom(romData, sourceMap) {
        if (!this.EmulatorClass) return;

        try {
            if (!this.emulator) this.emulator = new this.EmulatorClass();
            this.emulator.load_rom(romData);
            this.cancelFrame();
            this.releaseInputs();
            this.stopAudio();
            this.sourceMap = sourceMap;
            this.compiledProject = window.projectManager?.currentProject || null;
            // Audio is optional: an unavailable audio device must not prevent video/debugging.
            try {
                const AudioContext = window.AudioContext || window.webkitAudioContext;
                if (!this.audioContext && AudioContext) this.audioContext = new AudioContext();
                if (this.audioContext) {
                    this.emulator.set_sample_rate(this.audioContext.sampleRate);
                    this.resumeAudio();
                }
            } catch (e) { console.warn('Audio unavailable:', e); }

            // Attach PPU Viewer
            if(this.ppuViewer.attach) this.ppuViewer.attach(this.emulator);

            // Reset Input State
            this.keyboardState.fill(false);
            this.gamepadState.fill(false);
            this.lastSentState.fill(false);
            this.pressedKeys.clear();

            this.createEmulatorOverlay();

            // Sync initial breakpoints
            this.syncBreakpoints();

            // Start Loop
            this.emulatorRunning = true;
            this.updatePlayPauseButton();
            this.scheduleFrame();
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
            btnReset.onclick = () => this.resetEmulator();

            const btn1x = document.createElement('button');
            btn1x.innerText = '1x';
            btn1x.onclick = () => this.setScale(1);

            const btn2x = document.createElement('button');
            btn2x.innerText = '2x';
            btn2x.onclick = () => this.setScale(2);

            const btnFull = document.createElement('button');
            btnFull.innerText = 'Full';
            btnFull.onclick = () => this.setScale(0);

            const btnMem = document.createElement('button');
            btnMem.innerText = 'Memory';
            btnMem.onclick = () => this.toggleMemoryViewer();

            const btnPpu = document.createElement('button');
            btnPpu.innerText = 'PPU';
            btnPpu.onclick = () => this.ppuViewer.show();

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
                this.cancelFrame();
                this.releaseInputs();
                this.stopAudio();
                overlay.style.display = 'none';
                this.suspendAudio();
                this.clearDebugHighlight();
                if(this.ppuViewer.hide) this.ppuViewer.hide();
            };

            toolbar.append(btnPlay, btnReset, btn1x, btn2x, btnFull, btnMem, btnPpu, volContainer, btnClose);
            overlay.appendChild(toolbar);

            const mainArea = document.createElement('div');
            mainArea.style.display = 'flex';
            mainArea.style.gap = '20px';
            mainArea.style.alignItems = 'flex-start';

            const canvas = document.createElement('canvas');
            canvas.id = 'emulator-canvas';
            canvas.width = 256;
            canvas.height = 240;
            canvas.style.imageRendering = 'pixelated';
            canvas.style.border = '2px solid #fff';

            // Memory Viewer Container
            const memViewer = document.createElement('div');
            memViewer.id = 'memory-viewer';
            memViewer.style.display = 'none'; // Hidden by default
            memViewer.style.width = '420px';
            memViewer.style.height = '480px'; // 2x height approx
            memViewer.style.backgroundColor = '#1e1e1e';
            memViewer.style.color = '#0f0';
            memViewer.style.fontFamily = 'monospace';
            memViewer.style.fontSize = '12px';
            memViewer.style.padding = '10px';
            memViewer.style.overflowY = 'auto';
            memViewer.style.border = '1px solid #444';
            memViewer.innerHTML = '<div style="text-align:center">RAM Viewer ($0000-$07FF)</div><div id="mem-content"></div>';
            memViewer.addEventListener('scroll', () => this.updateMemoryView());

            mainArea.appendChild(canvas);
            mainArea.appendChild(memViewer);
            overlay.appendChild(mainArea);

            document.body.appendChild(overlay);
        } else {
            overlay.style.display = 'flex';
        }

        this.canvas = document.getElementById('emulator-canvas');
        this.ctx = this.canvas.getContext('2d');
        this.imageData = this.ctx.createImageData(256, 240);
        this.setScale(this.scale);

    }

    toggleMemoryViewer() {
        const mv = document.getElementById('memory-viewer');
        if (!mv) return;
        this.memoryViewerOpen = !this.memoryViewerOpen;
        mv.style.display = this.memoryViewerOpen ? 'block' : 'none';
        if (this.memoryViewerOpen) {
            this.updateMemoryView();
        }
    }

    updateMemoryView() {
        if (!this.memoryViewerOpen) return;
        const wram = this.getWRAM();
        if (!wram) return;

        const content = document.getElementById('mem-content');
        if (!content) return;

        let html = '<table cellpadding="0" cellspacing="0" style="width:100%; border-collapse:collapse;">';
        // Header
        html += '<tr><th style="color:#aaa">Addr</th>';
        for(let i=0; i<16; i++) {
            html += `<th style="color:#aaa">${i.toString(16).toUpperCase().padStart(2,'0')}</th>`;
        }
        html += '<th style="color:#aaa">Ascii</th></tr>';

        // Limit to first 2KB ($0000 - $07FF) for now, even if WRAM is larger
        // Tetanes WRAM usually includes 2KB internal + maybe save RAM?
        // Let's safe guard size
        const limit = Math.min(this.memorySize, wram.length);

        const viewer = document.getElementById('memory-viewer');
        const rowHeight = 18;
        const totalRows = Math.ceil(limit / 16);
        const visibleRows = Math.min(totalRows, Math.ceil((viewer?.clientHeight || 480) / rowHeight) + 4);
        const firstRow = Math.max(0, Math.min(totalRows - visibleRows, Math.floor((viewer?.scrollTop || 0) / rowHeight) - 2));
        // Fixed-height spacers preserve full RAM scrolling while formatting only
        // visible rows, even when a breakpoint causes an immediate refresh.
        if (firstRow) html += `<tr style="height:${firstRow * rowHeight}px"><td colspan="18"></td></tr>`;
        for (let r = firstRow * 16; r < Math.min(limit, (firstRow + visibleRows) * 16); r += 16) {
            html += `<tr style="height:${rowHeight}px;line-height:${rowHeight}px"><td style="color:#fe9">${r.toString(16).toUpperCase().padStart(4,'0')}</td>`;
            let ascii = '';
            for (let c = 0; c < 16; c++) {
                if (r + c < limit) {
                    const val = wram[r + c];
                    html += `<td>${val.toString(16).toUpperCase().padStart(2,'0')}</td>`;
                    ascii += (val >= 32 && val <= 126) ? String.fromCharCode(val) : '.';
                } else {
                    html += '<td></td>';
                }
            }
            html += `<td style="color:#aaa">${this.escapeHtml(ascii)}</td></tr>`;
        }
        const remainingRows = totalRows - firstRow - visibleRows;
        if (remainingRows > 0) html += `<tr style="height:${remainingRows * rowHeight}px"><td colspan="18"></td></tr>`;
        html += '</table>';
        content.innerHTML = html;
    }

    togglePause() {
        if (!this.emulator) return;
        this.emulatorRunning = !this.emulatorRunning;
        this.updatePlayPauseButton();
        if (this.emulatorRunning) {
             this.resumeAudio();
             this.lastFrameTime = null;
             this.frameDebt = 0;
             this.scheduleFrame();
        } else {
             this.cancelFrame();
             this.releaseInputs();
             this.stopAudio();
             this.suspendAudio();
             if (this.memoryViewerOpen) this.updateMemoryView();
             if (this.ppuViewer.isVisible) this.ppuViewer.update();
        }
    }

    resetEmulator() {
        if (!this.emulator) return;
        this.cancelFrame();
        this.releaseInputs();
        this.stopAudio();
        this.emulator.reset();
        this.lastSentState.fill(false);
        this.clearDebugHighlight();
        this.renderVideo();
        if (this.memoryViewerOpen) this.updateMemoryView();
        if (this.ppuViewer.isVisible) this.ppuViewer.update();
        this.scheduleFrame();
    }

    resumeAudio() {
        if (this.audioContext?.state === 'suspended') {
            Promise.resolve(this.audioContext.resume()).catch(e => console.warn('Audio resume failed:', e));
        }
    }

    suspendAudio() {
        if (this.audioContext?.state === 'running') {
            Promise.resolve(this.audioContext.suspend()).catch(e => console.warn('Audio suspend failed:', e));
        }
    }

    stopAudio() {
        // Scheduled sources outlive a frame; discard them at every execution boundary.
        for (const {source, gain} of this.audioSources || []) {
            source.onended = null;
            try { source.stop(); } catch (_) { /* It may already have ended. */ }
            source.disconnect();
            gain.disconnect();
        }
        this.audioSources?.clear();
        this.nextStartTime = this.audioContext?.currentTime || 0;
        this.emulator?.clear_audio_samples?.();
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
        const overlay = document.getElementById('emulator-overlay');
        if (!overlay || overlay.style.display === 'none' || !this.emulator) return;

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
            if (pressed) this.pressedKeys.add(e.code);
            else this.pressedKeys.delete(e.code);
            this.keyboardState[btn] = btn === 2
                ? this.pressedKeys.has('ShiftLeft') || this.pressedKeys.has('ShiftRight')
                : pressed;
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

        if (this.gamepadIndex === null) { this.gamepadState.fill(false); this.syncInputs(); return; }
        const gp = gamepads[this.gamepadIndex];
        if (!gp || !gp.connected) { this.gamepadIndex = null; this.gamepadState.fill(false); this.syncInputs(); return; }

        const b = gp.buttons;
        const axes = gp.axes;
        const PRESSED = (btn) => Boolean(btn && (typeof btn === 'object' ? btn.pressed : btn > 0.5));

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

    releaseInputs() {
        this.pressedKeys?.clear();
        this.keyboardState.fill(false);
        this.gamepadState.fill(false);
        this.syncInputs();
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

    getDebugState() {
        if (!this.emulator) return null;
        let s;
        try {
            s = this.emulator.get_cpu_state();
            const offset = this.emulator.prg_offset(s.pc);
            const res = {
                pc: s.pc, sp: s.sp, acc: s.acc, x: s.x, y: s.y, status: s.status, cycles: s.cycles,
                bank: offset == null || offset < 0 ? null : Math.floor(offset / 16384)
            };
            return res;
        } catch(e) { return null; }
        finally { s?.free?.(); }
    }

    getWRAM() {
        if (!this.emulator || !this.wasmMemory) return null;
        try {
            const ptr = this.emulator.get_wram();
            const len = this.emulator.get_wram_len();
            return new Uint8Array(this.wasmMemory.buffer, ptr, len);
        } catch(e) { return null; }
    }

    updateDebugInfo(pc) {
        const offset = this.emulator?.prg_offset(pc);
        if (offset == null || offset < 0) { this.clearDebugHighlight(); this.activeLine = -1; return; }
        const entry = this.mappedEntries().find(entry =>
            entry.bank === Math.floor(offset / 16384) && pc >= entry.cpu_start && pc < entry.cpu_end);
        if (entry) {
            if (entry.line !== this.activeLine) this.highlightDebugLine(entry.line);
            this.activeLine = entry.line;
        } else {
            this.clearDebugHighlight();
            this.activeLine = -1;
        }
    }
    highlightDebugLine(lineNum) {
        // Clear previous
        this.clearDebugHighlight();

        // Find element
        const el = this.lineNumbers.querySelector(`div[data-line="${lineNum}"]`);
        if (el) {
            el.classList.add('debug-active');
            // Auto scroll to keep in view if needed
            // Only if emulator is running slowly or paused, otherwise it jumps too much
            // For now, maybe just highlight
        }
    }

    clearDebugHighlight() {
        const prev = this.lineNumbers.querySelector('.debug-active');
        if (prev) prev.classList.remove('debug-active');
    }

    scheduleFrame() {
        if (!this.emulatorRunning || this.frameRequest !== null) return;
        this.frameRequest = requestAnimationFrame(time => {
            this.frameRequest = null;
            this.emulatorLoop(time);
        });
    }

    cancelFrame() {
        if (this.frameRequest !== null) cancelAnimationFrame(this.frameRequest);
        this.frameRequest = null;
        this.lastFrameTime = null;
        this.frameDebt = 0;
    }

    emulatorLoop(time) {
        if (!this.emulatorRunning) return;

        // Poll inputs
        this.pollGamepads();

        try {
            // Bound catch-up after slow frames; never accumulate a hidden-tab backlog.
            const frameMs = 1000 / this.emulator.frame_rate();
            if (this.lastFrameTime === null) this.lastFrameTime = time;
            this.frameDebt += Math.min(Math.max(time - this.lastFrameTime, 0), frameMs * 4);
            this.lastFrameTime = time;
            if (this.frameDebt < frameMs) { this.scheduleFrame(); return; }
            let breakpointHit = false;
            while (this.frameDebt >= frameMs && !breakpointHit) {
                breakpointHit = this.emulator.step();
                this.frameDebt -= frameMs;
            }

            // Debug Update (Every frame)
            const s = this.getDebugState();
            if (s) {
               this.updateDebugInfo(s.pc, s.bank);
            }

            // Update Memory Viewer if open
            if (this.memoryViewerOpen) {
                if (this.frameCount % 10 === 0) {
                     this.updateMemoryView();
                }
            }

            // Update PPU Viewer if open
            if (this.ppuViewer.isVisible) {
                 if (this.frameCount % 5 === 0) { // Throttle slightly
                     this.ppuViewer.update();
                 }
            }

            this.frameCount++;

            if (breakpointHit) {
                this.emulatorRunning = false;
                this.cancelFrame();
                this.releaseInputs();
                this.stopAudio();
                this.suspendAudio();
                this.updatePlayPauseButton();
                // Force update views on break
                if (this.memoryViewerOpen) this.updateMemoryView();
                this.ppuViewer.update();
                console.log("Breakpoint Hit!");
            }

            this.renderVideo();

            // Audio
            const audioPtr = this.emulator.get_audio_samples();
            const audioLen = this.emulator.get_audio_samples_len();
            if (this.emulatorRunning && audioLen > 0 && this.wasmMemory && this.audioContext && this.audioContext.state === 'running') {
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
                 const scheduled = {source, gain};
                 this.audioSources.add(scheduled);
                 source.onended = () => {
                     source.disconnect();
                     gain.disconnect();
                     this.audioSources.delete(scheduled);
                 };

                 if (this.nextStartTime < this.audioContext.currentTime) {
                     this.nextStartTime = this.audioContext.currentTime;
                 }
                 source.start(this.nextStartTime);
                 this.nextStartTime += buffer.duration;
            }
            // A suspended/missing device must not accumulate seconds of stale samples.
            this.emulator.clear_audio_samples?.();

            this.scheduleFrame();
        } catch (e) {
            console.error(e);
            this.emulatorRunning = false;
            this.cancelFrame();
            this.releaseInputs();
            this.stopAudio();
            this.suspendAudio();
            this.updatePlayPauseButton();
        }
    }

    renderVideo() {
        if (!this.emulator || !this.wasmMemory || !this.imageData || !this.ctx) return;
        const ptr = this.emulator.get_pixels();
        const len = this.emulator.get_pixels_len();
        const required = 256 * 240 * 4;
        if (len !== required || ptr < 0 || ptr + len > this.wasmMemory.buffer.byteLength) return;
        this.imageData.data.set(new Uint8Array(this.wasmMemory.buffer, ptr, len));
        this.ctx.putImageData(this.imageData, 0, 0);
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

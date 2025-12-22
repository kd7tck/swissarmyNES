// Basic Syntax Highlighter and Line Number Manager

class SwissEditor {
    constructor(editorId, highlightId, lineNumbersId) {
        this.editor = document.getElementById(editorId);
        this.highlight = document.getElementById(highlightId);
        this.lineNumbers = document.getElementById(lineNumbersId);
        this.emulator = null; // Store emulator instance

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
    }

    update() {
        const text = this.editor.value;

        // Update Line Numbers
        this.updateLineNumbers(text);

        // Update Highlighting
        this.updateHighlighting(text);
    }

    syncScroll() {
        this.highlight.scrollTop = this.editor.scrollTop;
        this.highlight.scrollLeft = this.editor.scrollLeft;
        this.lineNumbers.scrollTop = this.editor.scrollTop;
    }

    handleKey(e) {
        // Handle Tab key
        if (e.key === 'Tab') {
            e.preventDefault();
            const start = this.editor.selectionStart;
            const end = this.editor.selectionEnd;

            // Insert 2 spaces (or 4)
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
        // Simple regex-based highlighter
        // Note: This is purely visual and naively regenerates the whole HTML.

        // Escape HTML first
        // We do this by replacing special chars.
        // Note: This naive approach complicates tokenizing if we have "&amp;" in the text.
        // Better to tokenize original text then escape the parts.

        // Let's go line by line
        const lines = text.split('\n');
        const highlightedLines = lines.map(line => this.highlightLine(line));

        // Add a trailing newline if the text ends with one, to ensure the pre block grows
        if (text.endsWith('\n')) {
            highlightedLines.push('');
        }

        this.highlight.innerHTML = highlightedLines.join('<br>');
    }

    highlightLine(line) {
        if (!line) return '';

        // We need to parse the line into tokens: Strings, Comments, Keywords, Numbers, Others.
        // A simple state machine or robust regex is needed.
        // To handle "String with ' inside", we must match strings first or in parallel.

        // We will build chunks of (text, type)
        // Types: 'string', 'comment', 'keyword', 'number', 'plain'

        // Regex for tokens:
        // 1. String: "..." (non-greedy)
        // 2. Comment: '... or REM ... (until end of line)
        // 3. Number: $Hex, %Bin, Digits
        // 4. Keyword: \bWORD\b
        // 5. Operator/Punctuation

        const keywords = [
            'REM', 'BEGIN', 'END', 'NEXT', 'WEND', 'IF', 'THEN', 'ELSE',
            'SUB', 'INTERRUPT', 'ASM', 'ON', 'AS', 'DO', 'WHILE', 'FOR',
            'TO', 'STEP', 'LOOP', 'CONST', 'DIM', 'BYTE', 'WORD', 'BOOL',
            'PEEK', 'POKE', 'PRINT', 'RETURN', 'CALL', 'AND', 'OR', 'NOT',
            'LET', 'PLAY_SFX', 'DATA', 'READ', 'RESTORE', 'TYPE', 'ENUM',
            'SELECT', 'CASE', 'MACRO', 'METASPRITE', 'TILE', 'ANIMATION',
            'FRAME', 'WAIT_VBLANK'
        ];
        const keywordSet = new Set(keywords);

        // Regex explanation:
        // Group 1: String ("...")
        // Group 2: Comment ('...)
        // Group 3: Hex/Bin ($..., %...)
        // Group 4: Identifier/Keyword/Number ([a-zA-Z0-9_]+)
        // We process match by match.
        // Note: REM is a keyword but also starts a comment. Special handling needed.

        const tokenRegex = /(".*?")|('.*)|(\$[0-9A-Fa-f]+|%[01]+)|([a-zA-Z0-9_]+)|(.)/g;

        let resultHtml = '';
        let match;

        // Since we are processing line by line, comments end at end of string (which is end of line).

        while ((match = tokenRegex.exec(line)) !== null) {
            const [full, str, comment, number, word, other] = match;

            if (str) {
                resultHtml += `<span class="hl-string">${this.escapeHtml(str)}</span>`;
            } else if (comment) {
                resultHtml += `<span class="hl-comment">${this.escapeHtml(comment)}</span>`;
                // Once we hit a comment, the rest of the line is part of it (Regex '.* handles this)
            } else if (number) {
                 resultHtml += `<span class="hl-number">${this.escapeHtml(number)}</span>`;
            } else if (word) {
                // Check if keyword
                const upper = word.toUpperCase();
                if (upper === 'REM') {
                    // REM is a special case: it is a keyword that starts a comment.
                    // But our regex for comment only handled '
                    // So we treat REM as the start of a comment for the rest of the line.
                    // But wait, the regex loop continues.
                    // We need to consume the rest of the line if it is REM.

                    // Actually, if we match 'REM', we should mark it as comment start?
                    // Or keyword, then the rest is comment?
                    // Standard BASIC: REM is a statement.

                    // Let's look ahead.
                    // If we found 'REM', we treat it as a comment start.
                    // But we only matched "REM".
                    // The rest of the line is not consumed by this match.

                    // Easiest fix: modify regex to include REM.* as a comment group?
                    // But REM must be a whole word.

                    // If we found a word 'REM', we highlight it as comment (or keyword),
                    // and then we manually consume the rest of the line as comment.

                     resultHtml += `<span class="hl-comment">${this.escapeHtml(word)}</span>`;
                     // Consume rest of line
                     const rest = line.substring(tokenRegex.lastIndex);
                     resultHtml += `<span class="hl-comment">${this.escapeHtml(rest)}</span>`;
                     break; // Stop processing this line
                } else if (keywordSet.has(upper)) {
                     resultHtml += `<span class="hl-keyword">${this.escapeHtml(word)}</span>`;
                } else if (/^[0-9]+$/.test(word)) {
                     // Plain integer
                     resultHtml += `<span class="hl-number">${this.escapeHtml(word)}</span>`;
                } else {
                     // Identifier
                     resultHtml += this.escapeHtml(word);
                }
            } else if (other) {
                resultHtml += this.escapeHtml(other);
            }
        }

        return resultHtml;
    }

    escapeHtml(unsafe) {
        return unsafe
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;");
    }

    async runEmulator() {
        // 1. Compile Code (Reuse existing btn-compile logic if possible, or trigger it)
        // For now, let's assume we can trigger the compile button or use the compile API.
        // Ideally, app.js handles compilation and we get the binary.

        // But the Compile button in app.js probably just triggers compilation and maybe downloads the ROM?
        // We need to intercept the ROM data.

        // Let's dispatch an event that app.js listens to, or call a global function?
        // Better: trigger the "Compile" action and have it return the ROM data.

        // Let's look at app.js to see how compilation is handled.
        // For now, I'll just alert.
        console.log("Starting emulator...");

        // Load WASM if not loaded
        if (!this.wasmLoaded) {
            try {
                // Import the WASM module
                const module = await import('/wasm/swiss_emulator.js');
                await module.default(); // Initialize WASM
                this.EmulatorClass = module.Emulator;
                this.wasmLoaded = true;
                console.log("WASM loaded");
            } catch (e) {
                console.error("Failed to load WASM:", e);
                alert("Failed to load emulator core.");
                return;
            }
        }

        // We need the compiled ROM.
        // We can emit a custom event asking for the ROM.
        const event = new CustomEvent('request-compile-and-run');
        window.dispatchEvent(event);
    }

    startEmulatorWithRom(romData) {
        if (!this.EmulatorClass) return;

        if (this.emulator) {
             // Stop previous loop?
             // Since WASM memory management is manual, we might need to be careful.
             // But Emulator struct is dropped when replaced?
        }

        try {
            this.emulator = new this.EmulatorClass();
            this.emulator.load_rom(romData);

            // Create Canvas Overlay
            this.createEmulatorOverlay();

            // Start Loop
            this.emulatorRunning = true;
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
            overlay.style.position = 'fixed';
            overlay.style.top = '0';
            overlay.style.left = '0';
            overlay.style.width = '100vw';
            overlay.style.height = '100vh';
            overlay.style.backgroundColor = 'rgba(0,0,0,0.8)';
            overlay.style.display = 'flex';
            overlay.style.flexDirection = 'column';
            overlay.style.justifyContent = 'center';
            overlay.style.alignItems = 'center';
            overlay.style.zIndex = '1000';

            const canvas = document.createElement('canvas');
            canvas.id = 'emulator-canvas';
            canvas.width = 256;
            canvas.height = 240;
            canvas.style.width = '512px'; // 2x scale
            canvas.style.height = '480px';
            canvas.style.imageRendering = 'pixelated';
            canvas.style.border = '2px solid #fff';

            const closeBtn = document.createElement('button');
            closeBtn.innerText = "Close";
            closeBtn.style.marginTop = '10px';
            closeBtn.style.padding = '10px 20px';
            closeBtn.style.fontSize = '16px';
            closeBtn.onclick = () => {
                this.emulatorRunning = false;
                overlay.style.display = 'none';
            };

            overlay.appendChild(canvas);
            overlay.appendChild(closeBtn);
            document.body.appendChild(overlay);
        } else {
            overlay.style.display = 'flex';
        }

        this.canvas = document.getElementById('emulator-canvas');
        this.ctx = this.canvas.getContext('2d');
        this.imageData = this.ctx.createImageData(256, 240);

        // Bind keys
        document.addEventListener('keydown', (e) => this.handleEmulatorInput(e, true));
        document.addEventListener('keyup', (e) => this.handleEmulatorInput(e, false));
    }

    handleEmulatorInput(e, pressed) {
        if (!this.emulatorRunning || !this.emulator) return;

        // Map keys to NES buttons (Player 1)
        // A=Z, B=X, Select=Shift, Start=Enter, D-Pad=Arrows
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
            this.emulator.set_button(0, btn, pressed);
        }
    }

    emulatorLoop() {
        if (!this.emulatorRunning) return;

        try {
            this.emulator.step();

            const pixelsPtr = this.emulator.get_pixels();
            // 256 * 240 * 1 byte (palette index? or RGB?)
            // tetanes frame_buffer is likely RGB or RGBA?
            // tetanes-core by default might output raw indices or RGB depending on config.
            // But we didn't configure the renderer.
            // Let's assume tetanes-core output is 8-bit palette indices or 32-bit RGBA?
            // Actually, tetanes-core frame_buffer() doc says: "Returns the current frame buffer".
            // If it's pure core, it might be 256x240 pixels.

            // We need to know the format.
            // Standard tetanes-core often produces RGB/RGBA if PPU trait is used?
            // But wait, the PPU generates pixels.
            // Let's assume for now it's 256*240*something.

            const len = this.emulator.get_pixels_len();

            // Memory view
            // We need to access WASM memory.
            // This requires importing memory from module or accessing it via wasm-bindgen.
            // Usually wasm-bindgen handles TypedArrays if we return Vec<u8> or similar.
            // But we returned a pointer.

            // We need the WASM memory buffer.
            // module.wasm.memory.buffer?
            // This is accessible via `wasm_bindgen.memory`.

            const wasm = window.wasm_bindgen; // Global if using --no-modules, but we use ES modules?
            // With ES modules, the memory is exported.

            // Wait, how do we access memory if we use `import ...`?
            // `module.memory` should be exported if we use wasm-bindgen.

            // Let's assume I can get memory from the module object I imported.
            // But I imported `module` as the default export.

            // Actually, in `startEmulatorWithRom`:
            // const module = await import('/wasm/swiss_emulator.js');
            // await module.default();

            // The module object usually has `memory`.
            // Let's check `swiss_emulator.js` generated by wasm-bindgen later.
            // Usually `wasm` object is available.

            // IMPORTANT: If we use the default export (init), it usually returns the wasm instance which has exports.
            // Let's store the result of init.
        } catch (e) {
            console.error(e);
            this.emulatorRunning = false;
        }

        // Note: We need to properly verify pixel format.
        // For now, I will pause implementing the loop details until I verify what `get_pixels` returns.
        // I will just step for now.

        // requestAnimationFrame(() => this.emulatorLoop());
    }
}

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
    window.editor = new SwissEditor('code-editor', 'code-highlight', 'line-numbers');

    // Set some default text if empty
    const editor = document.getElementById('code-editor');
    if(editor && !editor.value) {
        editor.value = "CONST BG_COLOR = $0F\n\nSUB Main()\n  ' Set Background Color\n  POKE($2006, $3F)\n  POKE($2006, $00)\n  POKE($2007, BG_COLOR)\n  \n  DO\n    WAIT_VBLANK\n  LOOP\nEND SUB";
        window.editor.update();
    }
});

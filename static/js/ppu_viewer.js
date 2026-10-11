
class PpuViewer {
    constructor() {
        this.emulator = null;
        this.isVisible = false;
        this.animationFrameId = null;

        // Create UI elements dynamically or assume they are injected into a specific container
        this.container = document.createElement('div');
        this.container.id = 'ppu-viewer-container';
        this.container.style.display = 'none';
        this.container.style.position = 'fixed';
        this.container.style.top = '50px';
        this.container.style.right = '50px';
        this.container.style.width = '600px';
        this.container.style.height = '600px';
        this.container.style.backgroundColor = '#1e1e1e';
        this.container.style.border = '2px solid #444';
        // The viewer is created before the emulator overlay; equal levels put it behind.
        this.container.style.zIndex = '1100';
        this.container.style.flexDirection = 'column';
        this.container.style.padding = '10px';
        this.container.style.color = '#eee';
        this.container.style.boxShadow = '0 0 10px rgba(0,0,0,0.5)';
        this.container.style.overflow = 'auto';

        // Header
        const header = document.createElement('div');
        header.style.display = 'flex';
        header.style.justifyContent = 'space-between';
        header.style.marginBottom = '10px';
        const title = document.createElement('h3');
        title.innerText = 'PPU Debugger';
        title.style.margin = '0';
        const closeBtn = document.createElement('button');
        closeBtn.innerText = 'X';
        closeBtn.onclick = () => this.hide();
        header.appendChild(title);
        header.appendChild(closeBtn);
        this.container.appendChild(header);

        // Tabs
        const tabs = document.createElement('div');
        tabs.style.display = 'flex';
        tabs.style.gap = '10px';
        tabs.style.marginBottom = '10px';

        ['Pattern Tables', 'Nametables', 'Palettes', 'OAM'].forEach(name => {
            const btn = document.createElement('button');
            btn.innerText = name;
            btn.onclick = () => this.switchTab(name);
            tabs.appendChild(btn);
        });
        this.container.appendChild(tabs);

        // Content Areas
        this.contentArea = document.createElement('div');
        this.container.appendChild(this.contentArea);

        document.body.appendChild(this.container);

        this.currentTab = 'Pattern Tables';

        // Setup canvases/lists
        this.patternCanvas = document.createElement('canvas');
        this.patternCanvas.width = 256;
        this.patternCanvas.height = 128;
        this.patternCanvas.style.border = '1px solid #555';
        // Scaled up for visibility
        this.patternCanvas.style.width = '512px';
        this.patternCanvas.style.imageRendering = 'pixelated';

        this.nametableCanvas = document.createElement('canvas');
        this.nametableCanvas.width = 512;
        this.nametableCanvas.height = 480;
        this.nametableCanvas.style.border = '1px solid #555';
        this.nametableCanvas.style.width = '512px'; // Fit container
        this.nametableCanvas.style.imageRendering = 'pixelated';

        this.paletteCanvas = document.createElement('canvas');
        this.paletteCanvas.width = 16; // 32 RGBA palette entries, two rows.
        this.paletteCanvas.height = 2;
        this.paletteCanvas.style.imageRendering = 'pixelated';
        this.paletteCanvas.style.width = '100%';

        this.oamList = document.createElement('div');
        this.oamList.style.fontFamily = 'monospace';
        this.oamList.style.fontSize = '12px';
        this.oamList.style.whiteSpace = 'pre';
        this.oamList.style.overflowY = 'scroll';
        this.oamList.style.height = '400px';

        // Initial tab
        this.switchTab('Pattern Tables');
    }

    attach(emulator) {
        this.emulator = emulator;
        this.update();
    }

    show() {
        this.isVisible = true;
        this.container.style.display = 'flex';
        this.update();
    }

    hide() {
        this.isVisible = false;
        this.container.style.display = 'none';
        if(this.animationFrameId) {
            cancelAnimationFrame(this.animationFrameId);
        }
    }

    switchTab(name) {
        this.currentTab = name;
        this.contentArea.innerHTML = '';
        if (name === 'Pattern Tables') {
            this.contentArea.appendChild(this.patternCanvas);
        } else if (name === 'Nametables') {
            this.contentArea.appendChild(this.nametableCanvas);
        } else if (name === 'Palettes') {
            this.contentArea.appendChild(this.paletteCanvas);
        } else if (name === 'OAM') {
            this.contentArea.appendChild(this.oamList);
        }
        this.update();
    }

    update() {
        if (!this.isVisible || !this.emulator) return;

        // We only update if emulation is running or paused, usually handled by editor loop calling update()
        // but here we force update when showing.

        if (this.currentTab === 'Pattern Tables') {
            this.drawPatternTables();
        } else if (this.currentTab === 'Nametables') {
            this.drawNametables();
        } else if (this.currentTab === 'Palettes') {
            this.drawPalettes();
        } else if (this.currentTab === 'OAM') {
            this.drawOAM();
        }
    }

    drawPatternTables() {
        if (!this.emulator?.update_pattern_tables || !window.wasmMemory) return;
        this.emulator.update_pattern_tables();
        const ptr = this.emulator.get_pattern_tables();
        const len = this.emulator.get_pattern_tables_len();
        const memory = this.readBuffer(ptr, len, 256 * 128 * 4, 'pattern');
        if (!memory) return;
        // RGBA data
        const ctx = this.patternCanvas.getContext('2d');
        const imgData = ctx.createImageData(256, 128);
        imgData.data.set(memory);
        ctx.putImageData(imgData, 0, 0);
    }

    drawViewport(ctx) {
        const position = this.emulator?.viewport_position?.();
        if (!position || position.length !== 2) return;
        const x = position[0] % 512, y = position[1] % 480;
        ctx.strokeStyle = '#ff0';
        ctx.lineWidth = 2;
        // Clipped copies make nametable wrapping visible at both screen seams.
        for (const dx of [0, -512]) {
            for (const dy of [0, -480]) {
                ctx.strokeRect(x + dx, y + dy, 256, 240);
            }
        }
    }

    drawNametables() {
        if (!this.emulator?.update_nametables || !window.wasmMemory) return;
        this.emulator.update_nametables();
        const ptr = this.emulator.get_nametables();
        const len = this.emulator.get_nametables_len();
        const memory = this.readBuffer(ptr, len, 512 * 480 * 4, 'nametable');
        if (!memory) return;
        const ctx = this.nametableCanvas.getContext('2d');
        const imgData = ctx.createImageData(512, 480);
        imgData.data.set(memory);
        ctx.putImageData(imgData, 0, 0);
        this.drawViewport(ctx);
    }

    drawPalettes() {
        if (!this.emulator?.update_palettes || !window.wasmMemory) return;
        this.emulator.update_palettes();
        const ptr = this.emulator.get_palettes();
        const len = this.emulator.get_palettes_len();
        // The emulator exports exactly 32 RGBA entries in two rows of 16.
        const memory = this.readBuffer(ptr, len, 32 * 4, 'palette');
        if (!memory) return;
        const ctx = this.paletteCanvas.getContext('2d');
        this.paletteCanvas.width = 16;
        this.paletteCanvas.height = 2;
        const imgData = ctx.createImageData(16, 2);
        imgData.data.set(memory);
        ctx.putImageData(imgData, 0, 0);
        this.paletteCanvas.style.height = '50px';
        this.paletteCanvas.style.imageRendering = 'pixelated';
    }
    drawOAM() {
        if (!this.emulator?.get_oam_data || !window.wasmMemory) return;
        const ptr = this.emulator.get_oam_data();
        const len = this.emulator.get_oam_data_len();
        const memory = this.readBuffer(ptr, len, 256, 'OAM');

        this.oamList.innerHTML = PpuViewer.formatOamEntries(memory);
    }

    readBuffer(ptr, len, expected, name) {
        if (!window.wasmMemory || len === 0) return null;
        if (len !== expected) throw new Error(`Invalid ${name} buffer length`);
        // Recreate views after each core update because WASM memory may grow.
        const buffer = window.wasmMemory.buffer;
        if (!Number.isInteger(ptr) || ptr < 0 || ptr + len > buffer.byteLength) {
            throw new Error(`Invalid ${name} buffer bounds`);
        }
        return new Uint8Array(buffer, ptr, len);
    }

    static formatOamEntries(memory) {
        if (!memory || memory.length === 0) return 'No OAM data available';
        let html = '';
        for (let i = 0; i < 64; i++) {
            if (i * 4 + 3 >= memory.length) break;
            const y = memory[i * 4];
            const tile = memory[i * 4 + 1];
            const attr = memory[i * 4 + 2];
            const x = memory[i * 4 + 3];

            // OAM stores top minus one; 239 starts at the first offscreen scanline.
            const color = y >= 239 ? '#555' : '#fff';
            const hexTile = tile.toString(16).toUpperCase().padStart(2, '0');
            const binAttr = attr.toString(2).padStart(8, '0');
            html += `<div style="color:${color}">Sprite ${i}: X=${x} Y=${y} Tile=$${hexTile} Attr=${binAttr} Top=${y + 1} Palette=${attr & 3} Priority=${attr & 0x20 ? 'behind' : 'front'} Flip=${attr & 0x40 ? 'H' : '-'}${attr & 0x80 ? 'V' : '-'}</div>`;
        }
        return html;
    }

    static formatAttributeGrid(paletteData) {
        if (!paletteData || paletteData.length < 32) return '';
        let result = '';
        for (let bg = 0; bg < 4; bg++) {
            const sub = [];
            for (let c = 0; c < 4; c++) {
                const idx = bg * 4 + c;
                sub.push('$' + paletteData[idx].toString(16).toUpperCase().padStart(2, '0'));
            }
            result += `BG ${bg}: ${sub.join(' ')}\n`;
        }
        for (let sp = 0; sp < 4; sp++) {
            const sub = [];
            for (let c = 0; c < 4; c++) {
                const idx = 16 + sp * 4 + c;
                sub.push('$' + paletteData[idx].toString(16).toUpperCase().padStart(2, '0'));
            }
            result += `SP ${sp}: ${sub.join(' ')}\n`;
        }
        return result.trim();
    }
}


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
        this.container.style.zIndex = '1000';
        this.container.style.display = 'flex';
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
        this.paletteCanvas.width = 256; // Arbitrary width
        this.paletteCanvas.height = 32;
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
        if (!this.emulator.update_pattern_tables) return;
        this.emulator.update_pattern_tables();
        const ptr = this.emulator.get_pattern_tables();
        const len = this.emulator.get_pattern_tables_len();
        if (len === 0) return;

        const memory = new Uint8Array(window.wasmMemory.buffer, ptr, len);
        // RGBA data
        const ctx = this.patternCanvas.getContext('2d');
        const imgData = ctx.createImageData(256, 128);
        imgData.data.set(memory);
        ctx.putImageData(imgData, 0, 0);
    }

    drawNametables() {
        if (!this.emulator.update_nametables) return;
        this.emulator.update_nametables();
        const ptr = this.emulator.get_nametables();
        const len = this.emulator.get_nametables_len();
        if (len === 0) return;

        const memory = new Uint8Array(window.wasmMemory.buffer, ptr, len);
        const ctx = this.nametableCanvas.getContext('2d');
        const imgData = ctx.createImageData(512, 480);
        imgData.data.set(memory);
        ctx.putImageData(imgData, 0, 0);
    }

    drawPalettes() {
        if (!this.emulator.update_palettes) return;
        this.emulator.update_palettes();
        const ptr = this.emulator.get_palettes();
        const len = this.emulator.get_palettes_len();
        if (len === 0) return;

        const memory = new Uint8Array(window.wasmMemory.buffer, ptr, len);
        // The emulator exports exactly 32 RGBA entries in two rows of 16.
        if (len !== 32 * 4) throw new Error('Invalid palette buffer length');
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
        if (!this.emulator.get_oam_data) return;
        const ptr = this.emulator.get_oam_data();
        const len = this.emulator.get_oam_data_len();
        const memory = new Uint8Array(window.wasmMemory.buffer, ptr, len);

        this.oamList.innerHTML = PpuViewer.formatOamEntries(memory);
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

            // Highlight used sprites (Y < 240 usually)
            const color = y >= 240 ? '#555' : '#fff';
            const hexTile = tile.toString(16).toUpperCase().padStart(2, '0');
            const binAttr = attr.toString(2).padStart(8, '0');
            html += `<div style="color:${color}">Sprite ${i}: X=${x} Y=${y} Tile=$${hexTile} Attr=${binAttr}</div>`;
        }
        return html;
    }
}

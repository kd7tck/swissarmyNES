class MapEditor {
    constructor() {
        this.container = document.getElementById('map-editor-root');
        this.canvas = null;
        this.ctx = null;
        this.assets = null;

        this.targetType = 'nametable'; // 'nametable' or 'screen'
        this.currentIndex = 0;
        this.scale = 2; // 256x240 -> 512x480

        this.showGrid = true;
        this.currentPalette = 0; // 0-3
        this.mode = 'tile'; // 'tile', 'attribute', 'metatile' (Only applies to Nametables)

        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));
        window.addEventListener('palette-changed', () => this.render());
        window.addEventListener('chr-changed', () => this.render());

        this.init();
    }

    init() {
        if (!this.container) return;
        this.container.innerHTML = '';

        const controls = document.createElement('div');
        controls.className = 'map-controls';

        // Target Selector
        this.selTarget = document.createElement('select');
        this.selTarget.onchange = (e) => {
            const val = e.target.value;
            const parts = val.split(':');
            this.targetType = parts[0];
            this.currentIndex = parseInt(parts[1]);
            this.updateModeUI();
            this.render();
        };
        controls.appendChild(this.selTarget);

        // Add Buttons
        const btnAddNT = document.createElement('button');
        btnAddNT.textContent = '+ NT';
        btnAddNT.title = "Add Nametable";
        btnAddNT.onclick = () => this.addNametable();
        controls.appendChild(btnAddNT);

        const btnAddSC = document.createElement('button');
        btnAddSC.textContent = '+ Scr';
        btnAddSC.title = "Add Screen";
        btnAddSC.onclick = () => this.addScreen();
        controls.appendChild(btnAddSC);

        // Separator
        controls.appendChild(this.createSep());

        // Mode Toggle (Only for Nametables)
        this.btnMode = document.createElement('button');
        this.btnMode.textContent = 'Mode: Tiles';
        this.btnMode.onclick = () => this.toggleMode();
        controls.appendChild(this.btnMode);

         // Separator
         controls.appendChild(this.createSep());

        // Palette Selector
        const lblPal = document.createElement('span');
        lblPal.textContent = 'Palette: ';
        controls.appendChild(lblPal);

        for(let i=0; i<4; i++) {
            const btnPal = document.createElement('button');
            btnPal.textContent = `${i}`;
            btnPal.className = 'pal-btn';
            btnPal.onclick = () => {
                this.currentPalette = i;
                Array.from(controls.querySelectorAll('.pal-btn')).forEach(b => b.style.fontWeight = 'normal');
                btnPal.style.fontWeight = 'bold';
            };
            if (i === 0) btnPal.style.fontWeight = 'bold';
            controls.appendChild(btnPal);
        }

        // Separator
        controls.appendChild(this.createSep());

        // Grid Toggle
        const btnGrid = document.createElement('button');
        btnGrid.textContent = 'Grid';
        btnGrid.onclick = () => {
            this.showGrid = !this.showGrid;
            this.render();
        };
        controls.appendChild(btnGrid);

        this.container.appendChild(controls);

        // Canvas
        const wrapper = document.createElement('div');
        wrapper.className = 'map-canvas-wrapper';
        wrapper.style.overflow = 'auto';
        wrapper.style.maxHeight = '600px';

        this.canvas = document.createElement('canvas');
        this.canvas.width = 256 * this.scale;
        this.canvas.height = 240 * this.scale;
        this.canvas.className = 'map-canvas';
        this.ctx = this.canvas.getContext('2d');
        this.ctx.imageSmoothingEnabled = false;

        wrapper.appendChild(this.canvas);
        this.container.appendChild(wrapper);

        // Mouse Events
        let isDrawing = false;
        const handleMouse = (e, type) => {
            if (!this.assets) return;
            const rect = this.canvas.getBoundingClientRect();
            const x = Math.floor((e.clientX - rect.left) / this.scale);
            const y = Math.floor((e.clientY - rect.top) / this.scale);

            if (x >= 0 && x < 256 && y >= 0 && y < 240) {
                if (type === 'down') {
                    isDrawing = true;
                    this.handlePaint(x, y);
                } else if (type === 'move' && isDrawing) {
                    this.handlePaint(x, y);
                }
            }
        };
        this.canvas.addEventListener('mousedown', (e) => handleMouse(e, 'down'));
        this.canvas.addEventListener('mousemove', (e) => handleMouse(e, 'move'));
        window.addEventListener('mouseup', () => { isDrawing = false; });
    }

    createSep() {
        const sep = document.createElement('span');
        sep.style.width = '20px';
        sep.style.display = 'inline-block';
        return sep;
    }

    onProjectLoaded(assets) {
        this.assets = assets;
        if (!this.assets.nametables) this.assets.nametables = [];
        if (!this.assets.screens) this.assets.screens = [];

        // Ensure attrs for Nametables
        this.assets.nametables.forEach(nt => {
            if (!nt.attrs || nt.attrs.length !== 64) nt.attrs = new Array(64).fill(0);
        });

        // Default: Nametable 0 if exists, else add one
        if (this.assets.nametables.length === 0 && this.assets.screens.length === 0) {
            this.addNametable(); // Adds NT 0
        }

        // Update Selector
        this.updateSelector();

        // Select first available
        if (this.assets.nametables.length > 0) {
            this.targetType = 'nametable';
            this.currentIndex = 0;
        } else {
            this.targetType = 'screen';
            this.currentIndex = 0;
        }

        this.updateModeUI();
        this.render();
    }

    updateSelector() {
        this.selTarget.innerHTML = '';

        if (this.assets.nametables) {
            this.assets.nametables.forEach((nt, i) => {
                const opt = document.createElement('option');
                opt.value = `nametable:${i}`;
                opt.textContent = nt.name || `Nametable ${i}`;
                if (this.targetType === 'nametable' && this.currentIndex === i) opt.selected = true;
                this.selTarget.appendChild(opt);
            });
        }

        if (this.assets.screens) {
            this.assets.screens.forEach((sc, i) => {
                const opt = document.createElement('option');
                opt.value = `screen:${i}`;
                opt.textContent = sc.name || `Screen ${i}`;
                if (this.targetType === 'screen' && this.currentIndex === i) opt.selected = true;
                this.selTarget.appendChild(opt);
            });
        }
    }

    addNametable() {
        if (!this.assets) return;
        const newData = new Array(960).fill(0);
        const newAttrs = new Array(64).fill(0);
        this.assets.nametables.push({
            name: `Nametable ${this.assets.nametables.length}`,
            data: newData,
            attrs: newAttrs
        });
        this.targetType = 'nametable';
        this.currentIndex = this.assets.nametables.length - 1;
        this.updateSelector();
        this.updateModeUI();
        this.render();
    }

    addScreen() {
        if (!this.assets) return;
        // 16x15 = 240 bytes
        const newData = new Array(240).fill(0);
        this.assets.screens.push({
            name: `Screen ${this.assets.screens.length}`,
            data: newData
        });
        this.targetType = 'screen';
        this.currentIndex = this.assets.screens.length - 1;
        this.updateSelector();
        this.updateModeUI();
        this.render();
    }

    toggleMode() {
        if (this.targetType === 'screen') return; // Locked

        if (this.mode === 'tile') this.mode = 'attribute';
        else if (this.mode === 'attribute') this.mode = 'metatile';
        else this.mode = 'tile';

        this.updateModeUI();
        this.render();
    }

    updateModeUI() {
        if (this.targetType === 'screen') {
            this.btnMode.textContent = 'Mode: Screen (Metatiles)';
            this.btnMode.disabled = true;
            this.mode = 'metatile'; // Implicitly metatile mode
        } else {
            this.btnMode.disabled = false;
            let label = 'Tiles';
            if (this.mode === 'attribute') label = 'Attributes';
            if (this.mode === 'metatile') label = 'Metatiles';
            this.btnMode.textContent = `Mode: ${label}`;
        }
    }

    handlePaint(x, y) {
        if (this.targetType === 'nametable') {
            const tileX = Math.floor(x / 8);
            const tileY = Math.floor(y / 8);
            if (this.mode === 'tile') this.placeTile(tileX, tileY);
            else if (this.mode === 'attribute') this.placeAttribute(tileX, tileY);
            else if (this.mode === 'metatile') this.placeMetatileOnNametable(tileX, tileY);
        } else {
            // Screen
            const metaX = Math.floor(x / 16);
            const metaY = Math.floor(y / 16);
            this.placeMetatileOnScreen(metaX, metaY);
        }
    }

    placeTile(tx, ty) {
        if (!this.assets.nametables[this.currentIndex]) return;
        let tileIndex = window.chrEditor ? window.chrEditor.currentTileIndex : 0;
        const nt = this.assets.nametables[this.currentIndex];
        const idx = ty * 32 + tx;
        if (idx < nt.data.length && nt.data[idx] !== tileIndex) {
            nt.data[idx] = tileIndex;
            this.render();
        }
    }

    placeAttribute(tx, ty) {
        if (!this.assets.nametables[this.currentIndex]) return;
        const attrX = Math.floor(tx / 4);
        const attrY = Math.floor(ty / 4);
        const attrIdx = attrY * 8 + attrX;
        const nt = this.assets.nametables[this.currentIndex];

        // logic same as before
        const isRight = (tx % 4) >= 2;
        const isBottom = (ty % 4) >= 2;
        let shift = 0;
        if (isRight) shift += 2;
        if (isBottom) shift += 4;

        const mask = ~(0x03 << shift);
        const byte = (nt.attrs[attrIdx] & mask) | ((this.currentPalette & 0x03) << shift);

        if (nt.attrs[attrIdx] !== byte) {
            nt.attrs[attrIdx] = byte;
            this.render();
        }
    }

    placeMetatileOnNametable(tx, ty) {
        if (!window.metatileEditor || window.metatileEditor.currentMetatileIndex < 0) return;
        const metaIdx = window.metatileEditor.currentMetatileIndex;
        if (!this.assets.metatiles[metaIdx]) return;

        const meta = this.assets.metatiles[metaIdx];
        const nt = this.assets.nametables[this.currentIndex];

        const metaX = Math.floor(tx / 2) * 2;
        const metaY = Math.floor(ty / 2) * 2;

        // Set tiles
        nt.data[metaY * 32 + metaX] = meta.tiles[0];
        nt.data[metaY * 32 + metaX + 1] = meta.tiles[1];
        nt.data[(metaY + 1) * 32 + metaX] = meta.tiles[2];
        nt.data[(metaY + 1) * 32 + metaX + 1] = meta.tiles[3];

        // Set Attribute (use override)
        const oldPal = this.currentPalette;
        this.currentPalette = meta.attr;
        this.placeAttribute(metaX, metaY); // Updates 16x16 region attr
        this.currentPalette = oldPal;

        this.render();
    }

    placeMetatileOnScreen(mx, my) {
        if (!this.assets.screens[this.currentIndex]) return;
        if (mx >= 16 || my >= 15) return;

        if (!window.metatileEditor || window.metatileEditor.currentMetatileIndex < 0) return;
        const metaIdx = window.metatileEditor.currentMetatileIndex;

        const sc = this.assets.screens[this.currentIndex];
        const idx = my * 16 + mx;

        if (sc.data[idx] !== metaIdx) {
            sc.data[idx] = metaIdx;
            this.render();
        }
    }

    getAttribute(tx, ty) {
        // Only for Nametables
        if (this.targetType !== 'nametable') return 0;
        const nt = this.assets.nametables[this.currentIndex];
        if (!nt || !nt.attrs) return 0;

        const attrX = Math.floor(tx / 4);
        const attrY = Math.floor(ty / 4);
        const idx = attrY * 8 + attrX;
        if (idx >= nt.attrs.length) return 0;

        const isRight = (tx % 4) >= 2;
        const isBottom = (ty % 4) >= 2;
        let shift = 0;
        if (isRight) shift += 2;
        if (isBottom) shift += 4;

        return (nt.attrs[idx] >> shift) & 0x03;
    }

    render() {
        if (!this.ctx || !this.assets) return;

        // Clear
        this.ctx.fillStyle = '#000';
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        const chrBank = this.assets.chr_bank;

        // Get Sub-Palettes
        const subPalettes = [];
        for(let i=0; i<4; i++) {
             let colors = ['#000', '#555', '#aaa', '#fff'];
             if (window.paletteEditor && this.assets.palettes && this.assets.palettes[i]) {
                 colors = this.assets.palettes[i].colors.map(c => {
                    if (window.paletteEditor.nesPalette) return '#' + window.paletteEditor.nesPalette[c & 0x3F];
                    return '#fff';
                 });
             }
             subPalettes.push(colors);
        }

        if (this.targetType === 'nametable') {
            const nt = this.assets.nametables[this.currentIndex];
            if (!nt) return;

            for (let r = 0; r < 30; r++) {
                for (let c = 0; c < 32; c++) {
                    const tileIdx = nt.data[r * 32 + c];
                    const palIdx = this.getAttribute(c, r);
                    this.drawTile(c * 8, r * 8, tileIdx, chrBank, subPalettes[palIdx]);
                }
            }
        } else {
            // Screen
            const sc = this.assets.screens[this.currentIndex];
            if (!sc) return;

            for (let r = 0; r < 15; r++) {
                for (let c = 0; c < 16; c++) {
                    const metaIdx = sc.data[r * 16 + c];
                    const meta = this.assets.metatiles ? this.assets.metatiles[metaIdx] : null;
                    if (meta) {
                        const px = c * 16;
                        const py = r * 16;
                        // Draw 4 tiles
                        this.drawTile(px, py, meta.tiles[0], chrBank, subPalettes[meta.attr]);
                        this.drawTile(px+8, py, meta.tiles[1], chrBank, subPalettes[meta.attr]);
                        this.drawTile(px, py+8, meta.tiles[2], chrBank, subPalettes[meta.attr]);
                        this.drawTile(px+8, py+8, meta.tiles[3], chrBank, subPalettes[meta.attr]);
                    }
                }
            }
        }

        // Grid
        if (this.showGrid) {
            this.ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
            this.ctx.lineWidth = 1;
            this.ctx.beginPath();

            // Standard 8x8 grid or 16x16 depending on mode?
            // Nametable: 8x8 is useful.
            // Screen: 16x16 is the base unit.

            // Draw 8x8 first (faint)
            if (this.targetType === 'nametable') {
                 for (let c = 0; c <= 32; c++) {
                    this.ctx.moveTo(c * 8 * this.scale, 0);
                    this.ctx.lineTo(c * 8 * this.scale, 240 * this.scale);
                }
                for (let r = 0; r <= 30; r++) {
                    this.ctx.moveTo(0, r * 8 * this.scale);
                    this.ctx.lineTo(256 * this.scale, r * 8 * this.scale);
                }
                this.ctx.stroke();
            }

            // Draw 16x16 grid (stronger)
            this.ctx.beginPath();
            this.ctx.strokeStyle = 'rgba(0, 255, 0, 0.4)';
            this.ctx.lineWidth = 1;
             for (let c = 0; c <= 16; c++) {
                this.ctx.moveTo(c * 16 * this.scale, 0);
                this.ctx.lineTo(c * 16 * this.scale, 240 * this.scale);
             }
             for (let r = 0; r <= 15; r++) {
                this.ctx.moveTo(0, r * 16 * this.scale);
                this.ctx.lineTo(256 * this.scale, r * 16 * this.scale);
             }
            this.ctx.stroke();
        }
    }

    drawTile(x, y, tileIdx, chrBank, palette) {
        if (!chrBank) return;
        const tileOffset = tileIdx * 16;
        const scale = this.scale;

        for (let py = 0; py < 8; py++) {
            const lowByte = chrBank[tileOffset + py];
            const highByte = chrBank[tileOffset + py + 8];
            for (let px = 0; px < 8; px++) {
                const bitMask = 1 << (7 - px);
                const bit0 = (lowByte & bitMask) ? 1 : 0;
                const bit1 = (highByte & bitMask) ? 1 : 0;
                const colorVal = bit0 + (bit1 << 1);

                this.ctx.fillStyle = palette[colorVal];
                this.ctx.fillRect((x + px) * scale, (y + py) * scale, scale, scale);
            }
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('map-editor-root')) {
        window.mapEditor = new MapEditor();
    }
});

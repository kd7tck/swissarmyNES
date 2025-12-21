class MapEditor {
    constructor() {
        this.container = document.getElementById('map-editor-root');
        this.canvas = null;
        this.ctx = null;
        this.assets = null;
        this.currentNametableIndex = 0;
        this.scale = 2; // 256x240 -> 512x480

        // Default to showing grid
        this.showGrid = true;
        this.currentPalette = 0; // 0-3
        this.mode = 'tile'; // 'tile', 'attribute', 'metatile'

        // Cursor tracking
        this.mouseX = -1;
        this.mouseY = -1;

        // Listen for project load
        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));

        // Listen for palette changes to re-render
        window.addEventListener('palette-changed', () => this.render());

        // We also need to listen for CHR changes.
        window.addEventListener('chr-changed', () => this.render());

        // Listen for metatile changes to update instances
        window.addEventListener('metatile-changed', (e) => this.updateMetatileInstances(e.detail.index));

        this.init();
    }

    init() {
        if (!this.container) return;
        this.container.innerHTML = '';

        // Controls
        const controls = document.createElement('div');
        controls.className = 'map-controls';

        // Nametable Selector
        this.lblNametable = document.createElement('span');
        this.lblNametable.textContent = 'Nametable 0';
        controls.appendChild(this.lblNametable);

        // Add Nametable Button
        const btnAdd = document.createElement('button');
        btnAdd.textContent = '+';
        btnAdd.title = "Add Nametable";
        btnAdd.onclick = () => this.addNametable();
        controls.appendChild(btnAdd);

        // Separator
        const sep1 = document.createElement('span');
        sep1.style.width = '20px';
        sep1.style.display = 'inline-block';
        controls.appendChild(sep1);

        // Mode Toggle
        const btnMode = document.createElement('button');
        btnMode.textContent = 'Mode: Tiles';
        btnMode.onclick = () => {
            if (this.mode === 'tile') this.mode = 'attribute';
            else if (this.mode === 'attribute') this.mode = 'metatile';
            else this.mode = 'tile';

            let label = 'Tiles';
            if (this.mode === 'attribute') label = 'Attributes';
            if (this.mode === 'metatile') label = 'Metatiles';
            btnMode.textContent = `Mode: ${label}`;

            this.render(); // Redraw grid
        };
        controls.appendChild(btnMode);

         // Separator
         const sep2 = document.createElement('span');
         sep2.style.width = '20px';
         sep2.style.display = 'inline-block';
         controls.appendChild(sep2);

        // Palette Selector
        const lblPal = document.createElement('span');
        lblPal.textContent = 'Palette: ';
        controls.appendChild(lblPal);

        for(let i=0; i<4; i++) {
            const btnPal = document.createElement('button');
            btnPal.textContent = `${i}`;
            btnPal.onclick = () => {
                this.currentPalette = i;
                // Highlight active
                Array.from(controls.querySelectorAll('.pal-btn')).forEach(b => b.style.fontWeight = 'normal');
                btnPal.style.fontWeight = 'bold';
            };
            btnPal.className = 'pal-btn';
            if (i === 0) btnPal.style.fontWeight = 'bold';
            controls.appendChild(btnPal);
        }

        // Separator
        const sep3 = document.createElement('span');
        sep3.style.width = '20px';
        sep3.style.display = 'inline-block';
        controls.appendChild(sep3);

        // Grid Toggle
        const btnGrid = document.createElement('button');
        btnGrid.textContent = 'Grid';
        btnGrid.onclick = () => {
            this.showGrid = !this.showGrid;
            this.render();
        };
        controls.appendChild(btnGrid);

        this.container.appendChild(controls);

        // Canvas Wrapper
        const wrapper = document.createElement('div');
        wrapper.className = 'map-canvas-wrapper';
        wrapper.style.overflow = 'auto';
        wrapper.style.maxHeight = '600px';

        // Canvas
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
            if (!this.assets || !this.assets.nametables || this.assets.nametables.length === 0) return;

            // Coordinates are updated in mousemove listener below
            const x = this.mouseX;
            const y = this.mouseY;

            if (x >= 0 && x < 256 && y >= 0 && y < 240) {
                const tileX = Math.floor(x / 8);
                const tileY = Math.floor(y / 8);

                if (type === 'down') {
                    isDrawing = true;
                    if (this.mode === 'tile') {
                        this.placeTile(tileX, tileY);
                    } else if (this.mode === 'attribute') {
                        this.placeAttribute(tileX, tileY);
                    } else if (this.mode === 'metatile') {
                        this.placeMetatile(tileX, tileY);
                    }
                } else if (type === 'move' && isDrawing) {
                    if (this.mode === 'tile') {
                        this.placeTile(tileX, tileY);
                    } else if (this.mode === 'attribute') {
                        this.placeAttribute(tileX, tileY);
                    } else if (this.mode === 'metatile') {
                        this.placeMetatile(tileX, tileY);
                    }
                }
            }
        };

        this.canvas.addEventListener('mousedown', (e) => handleMouse(e, 'down'));
        this.canvas.addEventListener('mouseup', () => { isDrawing = false; });
        this.canvas.addEventListener('mouseleave', () => {
            isDrawing = false;
            this.mouseX = -1;
            this.mouseY = -1;
            if (this.mode === 'metatile') this.render();
        });

        this.canvas.addEventListener('mousemove', (e) => {
            const rect = this.canvas.getBoundingClientRect();
            this.mouseX = Math.floor((e.clientX - rect.left) / this.scale);
            this.mouseY = Math.floor((e.clientY - rect.top) / this.scale);

            if (this.mode === 'metatile') {
                this.render(); // Redraw for ghost
            }

            handleMouse(e, 'move');
        });

        // Picking
        this.canvas.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            this.handlePick();
        });
    }

    onProjectLoaded(assets) {
        this.assets = assets;
        if (!this.assets.nametables) {
            this.assets.nametables = [];
        }

        // Initialize attrs and metatile_grid if missing
        this.assets.nametables.forEach(nt => {
            if (!nt.attrs || nt.attrs.length !== 64) {
                nt.attrs = new Array(64).fill(0);
            }
            if (!nt.metatile_grid || nt.metatile_grid.length !== 240) {
                nt.metatile_grid = new Array(240).fill(-1);
            }
        });

        if (this.assets.nametables.length === 0) {
            this.addNametable();
        }

        this.currentNametableIndex = 0;
        this.render();
    }

    addNametable() {
        if (!this.assets) return;

        const newData = new Array(960).fill(0);
        const newAttrs = new Array(64).fill(0);
        const newMetaGrid = new Array(240).fill(-1);

        this.assets.nametables.push({
            name: `Nametable ${this.assets.nametables.length}`,
            data: newData,
            attrs: newAttrs,
            metatile_grid: newMetaGrid
        });

        this.currentNametableIndex = this.assets.nametables.length - 1;
        this.render();
    }

    handlePick() {
        if (!this.assets || !this.assets.nametables[this.currentNametableIndex]) return;
        if (this.mouseX < 0 || this.mouseY < 0) return;

        const tileX = Math.floor(this.mouseX / 8);
        const tileY = Math.floor(this.mouseY / 8);
        const nt = this.assets.nametables[this.currentNametableIndex];

        if (this.mode === 'metatile') {
            // Check metatile grid
            const metaX = Math.floor(tileX / 2);
            const metaY = Math.floor(tileY / 2);
            const gridIdx = metaY * 16 + metaX;

            if (nt.metatile_grid && gridIdx < nt.metatile_grid.length) {
                const metaIdx = nt.metatile_grid[gridIdx];
                if (metaIdx >= 0 && window.metatileEditor) {
                    window.metatileEditor.selectMetatile(metaIdx);
                }
            }
        } else if (this.mode === 'attribute') {
            const palIdx = this.getAttribute(tileX, tileY);
            this.currentPalette = palIdx;
            // Update UI buttons
            const btns = document.querySelectorAll('.pal-btn');
            btns.forEach((b, i) => {
                b.style.fontWeight = (i === palIdx) ? 'bold' : 'normal';
            });
        } else {
            // Tile mode
            const idx = tileY * 32 + tileX;
            if (idx < nt.data.length) {
                const tileVal = nt.data[idx];
                if (window.chrEditor) {
                    window.chrEditor.selectTile(tileVal);
                }
            }
        }
    }

    invalidateMetatileAt(tx, ty) {
        if (!this.assets || !this.assets.nametables[this.currentNametableIndex]) return;
        const nt = this.assets.nametables[this.currentNametableIndex];
        if (!nt.metatile_grid) return;

        const metaX = Math.floor(tx / 2);
        const metaY = Math.floor(ty / 2);
        const gridIdx = metaY * 16 + metaX;

        if (gridIdx < nt.metatile_grid.length) {
            nt.metatile_grid[gridIdx] = -1;
        }
    }

    placeTile(tx, ty) {
        if (!this.assets || !this.assets.nametables[this.currentNametableIndex]) return;

        // Get selected tile from CHR Editor
        let tileIndex = 0;
        if (window.chrEditor) {
            tileIndex = window.chrEditor.currentTileIndex;
        }

        const nt = this.assets.nametables[this.currentNametableIndex];
        const idx = ty * 32 + tx;

        if (idx < nt.data.length) {
            if (nt.data[idx] !== tileIndex) {
                nt.data[idx] = tileIndex;
                this.invalidateMetatileAt(tx, ty);
                this.render();
            }
        }
    }

    placeAttribute(tx, ty) {
        if (!this.assets || !this.assets.nametables[this.currentNametableIndex]) return;

        const attrX = Math.floor(tx / 4);
        const attrY = Math.floor(ty / 4);
        const attrIdx = attrY * 8 + attrX;

        if (attrIdx >= 64) return;

        const nt = this.assets.nametables[this.currentNametableIndex];
        let byte = nt.attrs[attrIdx];

        const isRight = (tx % 4) >= 2;
        const isBottom = (ty % 4) >= 2;

        let shift = 0;
        if (isRight) shift += 2;
        if (isBottom) shift += 4;

        // Mask out the old value
        const mask = ~(0x03 << shift);
        byte = (byte & mask) | ((this.currentPalette & 0x03) << shift);

        if (nt.attrs[attrIdx] !== byte) {
            nt.attrs[attrIdx] = byte;
            this.invalidateMetatileAt(tx, ty); // Attribute change invalidates metatile link too? Yes, strictly speaking.
            this.render();
        }
    }

    placeMetatile(tx, ty) {
        if (!this.assets || !this.assets.nametables[this.currentNametableIndex]) return;

        // Metatile mode works on 2x2 grid (16x16 pixel blocks).
        const metaX = Math.floor(tx / 2);
        const metaY = Math.floor(ty / 2);

        // Get current metatile
        if (!window.metatileEditor || window.metatileEditor.currentMetatileIndex < 0) return;
        if (!this.assets.metatiles || !this.assets.metatiles[window.metatileEditor.currentMetatileIndex]) return;

        const metaIdx = window.metatileEditor.currentMetatileIndex;
        const meta = this.assets.metatiles[metaIdx];
        const nt = this.assets.nametables[this.currentNametableIndex];

        // Update metatile grid
        if (nt.metatile_grid) {
            const gridIdx = metaY * 16 + metaX;
            if (gridIdx < nt.metatile_grid.length) {
                nt.metatile_grid[gridIdx] = metaIdx;
            }
        }

        this.applyMetatileToData(nt, metaX, metaY, meta);
        this.render();
    }

    applyMetatileToData(nt, metaX, metaY, meta) {
        // 1. Place 4 tiles
        // Top-Left
        const idxTL = (metaY * 2) * 32 + (metaX * 2);
        // Top-Right
        const idxTR = (metaY * 2) * 32 + (metaX * 2 + 1);
        // Bottom-Left
        const idxBL = (metaY * 2 + 1) * 32 + (metaX * 2);
        // Bottom-Right
        const idxBR = (metaY * 2 + 1) * 32 + (metaX * 2 + 1);

        if (idxTL < nt.data.length) nt.data[idxTL] = meta.tiles[0];
        if (idxTR < nt.data.length) nt.data[idxTR] = meta.tiles[1];
        if (idxBL < nt.data.length) nt.data[idxBL] = meta.tiles[2];
        if (idxBR < nt.data.length) nt.data[idxBR] = meta.tiles[3];

        // 2. Set Attribute
        // Use logic similar to placeAttribute but for the whole 2x2 block (which is one quadrant of an attribute byte, or full byte? No, attribute byte covers 4x4 tiles (32x32 pixels).)
        // A Metatile is 2x2 tiles (16x16 pixels).
        // So a Metatile is exactly one "attribute quadrant" (2 bits).

        // Coords in tiles
        const tx = metaX * 2;
        const ty = metaY * 2;

        const attrX = Math.floor(tx / 4);
        const attrY = Math.floor(ty / 4);
        const attrIdx = attrY * 8 + attrX;

        if (attrIdx >= 64) return;

        let byte = nt.attrs[attrIdx];

        const isRight = (tx % 4) >= 2;
        const isBottom = (ty % 4) >= 2;

        let shift = 0;
        if (isRight) shift += 2;
        if (isBottom) shift += 4;

        // Mask out the old value
        const mask = ~(0x03 << shift);
        byte = (byte & mask) | ((meta.attr & 0x03) << shift);

        nt.attrs[attrIdx] = byte;
    }

    updateMetatileInstances(metatileIndex) {
        if (!this.assets || !this.assets.nametables) return;

        let changed = false;
        const meta = this.assets.metatiles[metatileIndex];
        if (!meta) return;

        this.assets.nametables.forEach(nt => {
            if (!nt.metatile_grid) return;

            for (let i = 0; i < nt.metatile_grid.length; i++) {
                if (nt.metatile_grid[i] === metatileIndex) {
                    const metaX = i % 16;
                    const metaY = Math.floor(i / 16);
                    this.applyMetatileToData(nt, metaX, metaY, meta);
                    changed = true;
                }
            }
        });

        if (changed) this.render();
    }

    // Returns palette index (0-3) for a given tile coordinate
    getAttribute(tx, ty) {
         if (!this.assets || !this.assets.nametables[this.currentNametableIndex]) return 0;
         const nt = this.assets.nametables[this.currentNametableIndex];
         if (!nt.attrs) return 0;

         const attrX = Math.floor(tx / 4);
         const attrY = Math.floor(ty / 4);
         const attrIdx = attrY * 8 + attrX;

         if (attrIdx >= nt.attrs.length) return 0;

         const byte = nt.attrs[attrIdx];

         const isRight = (tx % 4) >= 2;
         const isBottom = (ty % 4) >= 2;

         let shift = 0;
         if (isRight) shift += 2;
         if (isBottom) shift += 4;

         return (byte >> shift) & 0x03;
    }

    render() {
        if (!this.ctx) return;

        // Clear
        this.ctx.fillStyle = '#000';
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);

        if (!this.assets || !this.assets.nametables || this.assets.nametables.length === 0) return;

        const nt = this.assets.nametables[this.currentNametableIndex];
        const chrBank = this.assets.chr_bank;

        // Precompute palettes
        const subPalettes = [];
        if (window.paletteEditor && this.assets.palettes) {
             for(let i=0; i<4; i++) {
                 let colors = ['#000000', '#666666', '#aaaaaa', '#ffffff'];
                 if (this.assets.palettes && this.assets.palettes[i]) {
                     colors = this.assets.palettes[i].colors.map(c => {
                        if (window.paletteEditor && window.paletteEditor.nesPalette) {
                            return '#' + window.paletteEditor.nesPalette[c & 0x3F];
                        }
                        return '#fff';
                     });
                 }
                 subPalettes.push(colors);
             }
        } else {
             // Fallback
             for(let i=0; i<4; i++) subPalettes.push(['#000', '#555', '#aaa', '#fff']);
        }

        // Render Tiles
        for (let r = 0; r < 30; r++) {
            for (let c = 0; c < 32; c++) {
                const tileIdx = nt.data[r * 32 + c];
                const palIdx = this.getAttribute(c, r);
                this.drawTile(c * 8, r * 8, tileIdx, chrBank, subPalettes[palIdx]);
            }
        }

        // Ghost Cursor for Metatile
        if (this.mode === 'metatile' && this.mouseX >= 0 && this.mouseY >= 0) {
            if (window.metatileEditor && window.metatileEditor.currentMetatileIndex >= 0) {
                const metaIdx = window.metatileEditor.currentMetatileIndex;
                const meta = this.assets.metatiles[metaIdx];
                if (meta) {
                    const tileX = Math.floor(this.mouseX / 8);
                    const tileY = Math.floor(this.mouseY / 8);

                    // Snap to 2x2 grid
                    const startX = Math.floor(tileX / 2) * 2;
                    const startY = Math.floor(tileY / 2) * 2;

                    this.ctx.save();
                    this.ctx.globalAlpha = 0.5;

                    const pal = subPalettes[meta.attr];

                    // Draw the 4 tiles of metatile
                    this.drawTile(startX * 8, startY * 8, meta.tiles[0], chrBank, pal);
                    this.drawTile((startX+1) * 8, startY * 8, meta.tiles[1], chrBank, pal);
                    this.drawTile(startX * 8, (startY+1) * 8, meta.tiles[2], chrBank, pal);
                    this.drawTile((startX+1) * 8, (startY+1) * 8, meta.tiles[3], chrBank, pal);

                    this.ctx.restore();

                    // Highlight box
                    this.ctx.strokeStyle = 'yellow';
                    this.ctx.lineWidth = 2;
                    this.ctx.strokeRect(startX * 8 * this.scale, startY * 8 * this.scale, 16 * this.scale, 16 * this.scale);
                }
            }
        }

        // Draw Grid
        if (this.showGrid) {
            this.ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
            this.ctx.lineWidth = 1;
            this.ctx.beginPath();

            // Vertical lines
            for (let c = 0; c <= 32; c++) {
                this.ctx.moveTo(c * 8 * this.scale, 0);
                this.ctx.lineTo(c * 8 * this.scale, 240 * this.scale);
            }

            // Horizontal lines
            for (let r = 0; r <= 30; r++) {
                this.ctx.moveTo(0, r * 8 * this.scale);
                this.ctx.lineTo(256 * this.scale, r * 8 * this.scale);
            }

            // If in Attribute or Metatile Mode, draw coarser grid (16x16 pixels -> 2x2 tiles)
            if (this.mode === 'attribute' || this.mode === 'metatile') {
                 this.ctx.stroke(); // Draw normal grid first

                 this.ctx.beginPath();
                 this.ctx.strokeStyle = 'rgba(0, 255, 0, 0.5)';
                 this.ctx.lineWidth = 2;

                 // Vertical attr lines (every 16 pixels)
                 for (let c = 0; c <= 16; c++) {
                    this.ctx.moveTo(c * 16 * this.scale, 0);
                    this.ctx.lineTo(c * 16 * this.scale, 240 * this.scale);
                 }
                 // Horizontal attr lines
                 for (let r = 0; r <= 15; r++) {
                    this.ctx.moveTo(0, r * 16 * this.scale);
                    this.ctx.lineTo(256 * this.scale, r * 16 * this.scale);
                 }
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
                this.ctx.fillRect(
                    (x + px) * scale,
                    (y + py) * scale,
                    scale, scale
                );
            }
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    if (document.getElementById('map-editor-root')) {
        window.mapEditor = new MapEditor();
    }
});

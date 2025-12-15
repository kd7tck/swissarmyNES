
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
        this.mode = 'tile'; // 'tile' or 'attribute'

        // Listen for project load
        window.addEventListener('project-loaded', (e) => this.onProjectLoaded(e.detail.assets));

        // Listen for palette changes to re-render
        window.addEventListener('palette-changed', () => this.render());

        // We also need to listen for CHR changes.
        window.addEventListener('chr-changed', () => this.render());

        this.init();
    }

    init() {
        if (!this.container) return;
        this.container.innerHTML = '';

        // Controls
        const controls = document.createElement('div');
        controls.className = 'map-controls';

        // Nametable Selector (placeholder for now)
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
            this.mode = this.mode === 'tile' ? 'attribute' : 'tile';
            btnMode.textContent = `Mode: ${this.mode === 'tile' ? 'Tiles' : 'Attributes'}`;
            // Optional: Visually indicate mode on the canvas?
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

            const rect = this.canvas.getBoundingClientRect();
            const x = Math.floor((e.clientX - rect.left) / this.scale);
            const y = Math.floor((e.clientY - rect.top) / this.scale);

            if (x >= 0 && x < 256 && y >= 0 && y < 240) {
                // Determine block coordinates for attributes (16x16 pixels -> 32x30 tiles -> 16x15 blocks)
                // Actually 256/16 = 16 blocks wide, 240/16 = 15 blocks tall.
                // Tile coords: 0-31, 0-29.

                const tileX = Math.floor(x / 8);
                const tileY = Math.floor(y / 8);

                if (type === 'down') {
                    isDrawing = true;
                    if (this.mode === 'tile') {
                        this.placeTile(tileX, tileY);
                    } else {
                        this.placeAttribute(tileX, tileY);
                    }
                } else if (type === 'move' && isDrawing) {
                    if (this.mode === 'tile') {
                        this.placeTile(tileX, tileY);
                    } else {
                        this.placeAttribute(tileX, tileY);
                    }
                }
            }
        };

        this.canvas.addEventListener('mousedown', (e) => handleMouse(e, 'down'));
        this.canvas.addEventListener('mousemove', (e) => handleMouse(e, 'move'));
        window.addEventListener('mouseup', () => { isDrawing = false; });
    }

    onProjectLoaded(assets) {
        this.assets = assets;
        if (!this.assets.nametables) {
            this.assets.nametables = [];
        }

        // Initialize attrs if missing for existing nametables
        this.assets.nametables.forEach(nt => {
            if (!nt.attrs || nt.attrs.length !== 64) {
                nt.attrs = new Array(64).fill(0);
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

        this.assets.nametables.push({
            name: `Nametable ${this.assets.nametables.length}`,
            data: newData,
            attrs: newAttrs
        });

        this.currentNametableIndex = this.assets.nametables.length - 1;
        this.render();
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
                this.render();
            }
        }
    }

    placeAttribute(tx, ty) {
        if (!this.assets || !this.assets.nametables[this.currentNametableIndex]) return;

        // Attribute blocks are 2x2 tiles (16x16 pixels).
        // Each byte in attribute table covers a 4x4 tile area (32x32 pixels).
        // A byte is split into 4 pairs of bits:
        //  - bits 0,1: top-left 2x2 block
        //  - bits 2,3: top-right 2x2 block
        //  - bits 4,5: bottom-left 2x2 block
        //  - bits 6,7: bottom-right 2x2 block

        // Grid is 32x30 tiles.
        // Attribute table is 8x8 bytes (64 bytes).
        // Each byte covers 4x4 tiles.
        // x coord in attr table (0-7) = tx / 4
        // y coord in attr table (0-7) = ty / 4

        const attrX = Math.floor(tx / 4);
        const attrY = Math.floor(ty / 4);
        const attrIdx = attrY * 8 + attrX;

        if (attrIdx >= 64) return;

        const nt = this.assets.nametables[this.currentNametableIndex];
        let byte = nt.attrs[attrIdx];

        // Determine which quadrant of the 4x4 block we are in.
        // tx % 4 tells us where we are in the 4-tile wide block.
        // 0,1 -> left, 2,3 -> right
        // ty % 4
        // 0,1 -> top, 2,3 -> bottom

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
            this.render();
        }
    }

    // Returns palette index (0-3) for a given tile coordinate
    getAttribute(tx, ty) {
         if (!this.assets || !this.assets.nametables[this.currentNametableIndex]) return 0;
         const nt = this.assets.nametables[this.currentNametableIndex];
         if (!nt.attrs) return 0;

         const attrX = Math.floor(tx / 4);
         const attrY = Math.floor(ty / 4);
         const attrIdx = attrY * 8 + attrX;

         if (attrIdx >= nt.attrs.length) return 0; // Out of bounds (e.g. bottom row 30 is handled by last attr byte usually but partial)
         // Wait, 30 rows / 4 = 7.5. The last row of attributes covers rows 28, 29, 30, 31.
         // 240 pixels / 16 = 15 attribute blocks vertically.
         // 15 attribute blocks / 2 (per byte) = 7.5 bytes.
         // The attribute table is 64 bytes (8x8).
         // 8 bytes * 4 tiles/byte = 32 tiles width. Correct.
         // 8 bytes * 4 tiles/byte = 32 tiles height.
         // Screen is only 30 tiles high. The last attribute row is partially used.

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
        // We need 4 sub-palettes
        const subPalettes = [];
        if (window.paletteEditor && this.assets.palettes) {
             // We assume pallets 0-3 correspond to index 0-3 in the palettes array?
             // Or does the Palette Editor manage a single "System Palette" composed of 4 subpalettes?
             // Checking palette.js might be needed. Usually "Palette Editor" edits the 4 BG and 4 Sprite palettes.
             // If assets.palettes is a list of named palettes, we need to know which one is active or "The Project Palette".
             // DESIGN.md says "Sub-palettes: Interface to assign colors to the 4 background and 4 sprite sub-palettes."
             // Let's assume assets.palettes[0-3] are the BG palettes.

             // If the user has created palettes, let's try to find 4 of them or use defaults.
             // Currently project.js defines 'palettes' as Vec<Palette>.

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

            // If in Attribute Mode, draw coarser grid for attributes (16x16 pixels -> 2x2 tiles)
            if (this.mode === 'attribute') {
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

                // Transparent pixel (color 0) typically renders the background color (palette[0]),
                // which is usually the universal background color.
                // For the editor, we just draw opaque.

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
